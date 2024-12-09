use super::{Answer, DateSelectionStrategy, Poll, PollOption, PollStage};
use crate::database::Repository;
use crate::event::PlanningDetails;
use crate::users::User;
use anyhow::Result;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rocket::tokio::sync::Mutex;
use std::sync::Arc;

mod scheduling;
pub(crate) use scheduling::*;
mod emails;
pub(crate) use emails::*;
use veto::veto_date_in_other_polls;
mod veto;

async fn finalize(ctx: &mut FinalizeContext) -> Result<()> {
    // not using a transaction here because we're the only ones setting polls to closed.
    while let Some(poll) = get_next_pending_poll(&mut *ctx.repository).await? {
        try_finalize_poll(ctx, poll).await?;
    }

    Ok(())
}

struct FinalizeContext {
    repository: Box<dyn Repository>,
    sender: Arc<Mutex<EventEmailSender>>,
}

async fn try_finalize_poll(ctx: &mut FinalizeContext, poll: Poll) -> Result<()> {
    ctx.repository
        .update_poll_stage(poll.id, PollStage::Finalizing)
        .await?;

    let result = finalize_poll_dry_run(&poll);

    if let FinalizeResult::Success {
        details,
        invited,
        missed,
        ..
    } = result
    {
        let event = ctx.repository.plan_event(poll.event.id, details).await?;
        veto_date_in_other_polls(ctx, &event).await?;
        let sender = &mut *ctx.sender.lock().await;
        emails::send_notification_emails(sender, &event, &invited, &missed).await?;
    }

    ctx.repository
        .update_poll_stage(poll.id, PollStage::Closed)
        .await?;

    Ok(())
}

async fn get_next_pending_poll(repository: &mut dyn Repository) -> Result<Option<Poll>> {
    Ok(repository
        .get_stateful_events()
        .await?
        .into_iter()
        .filter_map(|e| e.pending())
        .sorted_by_key(|p| p.open_until)
        .next())
}

fn finalize_poll_dry_run(poll: &Poll) -> FinalizeResult {
    let candidates = get_candidates(poll);
    if let Some(chosen_option) = choose_option(candidates, poll) {
        let invited = choose_participants(&chosen_option.answers);
        let details = PlanningDetails::new(&chosen_option, &invited);
        FinalizeResult::Success {
            missed: get_missed_users(poll, &invited),
            details,
            invited,
        }
    } else {
        FinalizeResult::Failure
    }
}

#[derive(Debug)]
enum FinalizeResult {
    /// Date selected, some people might not be invited though.
    Success {
        details: PlanningDetails,
        invited: Vec<User>,
        missed: Vec<User>,
    },
    /// No date found because there weren't enough people.
    Failure,
}

fn get_candidates(poll: &Poll) -> Vec<PollOption> {
    poll.options
        .iter()
        .filter(|o| !o.has_veto())
        .filter(|o| o.count_yes_answers() >= poll.min_participants)
        .cloned()
        .collect()
}

fn get_missed_users(poll: &Poll, invited: &[User]) -> Vec<User> {
    poll.options
        .iter()
        .flat_map(|o| o.answers.iter())
        .map(|a| &a.user)
        .unique_by(|u| u.id)
        .filter(|u| !invited.iter().any(|i| i.id == u.id))
        .cloned()
        .collect()
}

fn choose_option(mut candidates: Vec<PollOption>, poll: &Poll) -> Option<PollOption> {
    use DateSelectionStrategy::*;
    match poll.strategy {
        AtRandom => candidates.choose(&mut thread_rng()).cloned(),
        ToMaximizeParticipants => {
            // There are potentially multiple poll options that have
            // a "maximal" number of participants so we choose between all maximal
            // options at random.
            if let Some(max) = max_participants(&candidates) {
                candidates.retain(|o| (o.count_yes_answers()) >= max);
            }
            candidates.choose(&mut thread_rng()).cloned()
        }
    }
}

fn max_participants(options: &[PollOption]) -> Option<usize> {
    options.iter().map(|o| o.count_yes_answers()).max()
}

fn choose_participants(answers: &[Answer]) -> Vec<User> {
    answers.iter().filter_map(|a| a.yes()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Materialized;
    use crate::poll::{AnswerValue, Attendance};
    use crate::users::{EmailSubscription, Role, UserId};
    use time::OffsetDateTime;

    mod choose_participants {
        use super::*;
        use crate::poll::Attendance;

        const MAX_ALLOWED_PARTICIPANTS: usize = 5;

        #[test]
        fn participants_are_empty_for_empty_answers() {
            let invited = choose_participants(&[]);
            assert!(invited.is_empty());
        }

        #[test]
        fn chooses_all_with_yes_answer() {
            let invited = choose_participants(&[
                answer(AnswerValue::yes(Attendance::Required), UserId(1)),
                answer(AnswerValue::yes(Attendance::Optional), UserId(2)),
                answer(AnswerValue::yes(Attendance::Required), UserId(3)),
            ]);
            assert_eq!(
                vec![UserId(1), UserId(2), UserId(3)],
                invited.into_iter().map(|u| u.id).collect::<Vec<_>>()
            );
        }

        // TODO: more tests
    }

    mod max_participants {
        use super::*;

        #[test]
        fn max_is_none_if_options_are_empty() {
            assert!(max_participants(&[]).is_none());
        }

        #[test]
        fn max_is_max_of_all_yes_answers() {
            assert_eq!(
                Some(4),
                max_participants(&[poll_option(1, 0), poll_option(2, 1), poll_option(3, 4)],)
            );
        }
    }

    fn poll_option(id: i64, yes_answers: usize) -> PollOption<Materialized> {
        PollOption {
            id,
            starts_at: OffsetDateTime::now_utc().into(),
            answers: (0..yes_answers)
                .map(|i| answer(AnswerValue::yes(Attendance::Optional), UserId(i as i64)))
                .collect(),
        }
    }

    fn answer(value: AnswerValue, user: UserId) -> Answer<Materialized> {
        Answer {
            value,
            id: user.0,
            user: user_stub(user),
        }
    }

    fn user_stub(id: UserId) -> User {
        User {
            id,
            name: String::default(),
            role: Role::default(),
            email_address: String::default(),
            email_subscription: EmailSubscription::default(),
            invited_by: None,
            campaign: None,
            can_update_name: true,
            can_answer_strongly: true,
            last_active_at: OffsetDateTime::now_utc().into(),
        }
    }
}

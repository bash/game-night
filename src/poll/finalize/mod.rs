use super::{Answer, Attendance, DateSelectionStrategy, Poll, PollOption, PollStage};
use crate::database::Repository;
use crate::event::PlanningDetails;
use crate::users::User;
use anyhow::Result;
use itertools::{Either, Itertools};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rocket::tokio::sync::Mutex;
use std::cmp::min;
use std::sync::Arc;

mod scheduling;
pub(crate) use scheduling::*;
mod emails;
pub(crate) use emails::*;

async fn finalize(ctx: &mut FinalizeContext) -> Result<()> {
    // not using a transaction here because we're the only ones setting polls to closed.
    for poll in ctx.repository.get_polls_pending_for_finalization().await? {
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
        let sender = &mut *ctx.sender.lock().await;
        emails::send_notification_emails(sender, &event, &invited, &missed).await?;
    }

    ctx.repository
        .update_poll_stage(poll.id, PollStage::Closed)
        .await?;

    // TODO: veto selected date in open polls of other events.

    Ok(())
}

fn finalize_poll_dry_run(poll: &Poll) -> FinalizeResult {
    let candidates = get_candidates(poll);
    if let Some(chosen_option) = choose_option(candidates, poll) {
        let (invited, overflow) =
            choose_participants(&chosen_option.answers, poll.max_participants);
        let details = PlanningDetails::new(&chosen_option, &invited);
        FinalizeResult::Success {
            missed: get_missed_users(poll, &invited),
            details,
            invited,
            overflow,
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
        overflow: Vec<User>,
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
            if let Some(max) = max_participants(&candidates, poll.max_participants) {
                candidates.retain(|o| (o.count_yes_answers()) >= max);
            }
            candidates.choose(&mut thread_rng()).cloned()
        }
    }
}

fn max_participants(options: &[PollOption], max_allowed_participants: usize) -> Option<usize> {
    options
        .iter()
        .map(|o| o.count_yes_answers())
        .max()
        .map(|max| min(max, max_allowed_participants))
}

fn choose_participants(answers: &[Answer], max_participants: usize) -> (Vec<User>, Vec<User>) {
    let (mut accepted, mut rejected): (Vec<_>, Vec<_>) = pre_partition_by_attendance(answers);

    let available = max_participants.saturating_sub(accepted.len());
    if available > 0 {
        rejected.shuffle(&mut thread_rng());
        accepted.extend(rejected.drain(..min(available, rejected.len())));
    }

    (accepted, rejected)
}

fn pre_partition_by_attendance(answers: &[Answer]) -> (Vec<User>, Vec<User>) {
    answers
        .iter()
        .filter_map(|a| a.yes())
        .partition_map(|a| match a.0 {
            Attendance::Required => Either::Left(a.1),
            Attendance::Optional => Either::Right(a.1),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Materialized;
    use crate::poll::AnswerValue;
    use crate::users::{EmailSubscription, Role, UserId};
    use time::OffsetDateTime;

    mod choose_participants {
        use super::*;

        const MAX_ALLOWED_PARTICIPANTS: usize = 5;

        #[test]
        fn accepted_and_rejected_participants_are_empty_for_empty_answers() {
            let (accepted, rejected) = choose_participants(&[], MAX_ALLOWED_PARTICIPANTS);
            assert!(accepted.is_empty());
            assert!(rejected.is_empty());
        }

        #[test]
        fn accepts_more_than_max_if_all_have_required_attendance() {
            let (accepted, rejected) = choose_participants(
                &[
                    answer(AnswerValue::yes(Attendance::Required), UserId(1)),
                    answer(AnswerValue::yes(Attendance::Required), UserId(2)),
                    answer(AnswerValue::yes(Attendance::Required), UserId(3)),
                ],
                2,
            );
            assert_eq!(
                vec![UserId(1), UserId(2), UserId(3)],
                accepted.into_iter().map(|u| u.id).collect::<Vec<_>>()
            );
            assert!(rejected.is_empty());
        }

        // TODO: more tests
    }

    mod max_participants {
        use super::*;

        const MAX_ALLOWED_PARTICIPANTS: usize = 5;

        #[test]
        fn max_is_none_if_options_are_empty() {
            assert!(max_participants(&[], MAX_ALLOWED_PARTICIPANTS).is_none());
        }

        #[test]
        fn max_is_max_of_all_yes_answers() {
            assert_eq!(
                Some(4),
                max_participants(
                    &[poll_option(1, 0), poll_option(2, 1), poll_option(3, 4)],
                    MAX_ALLOWED_PARTICIPANTS
                )
            );
        }

        // This is important to ensure that options that have at least MAX_ALLOWED_PARTICIPANTS participants
        // but not overall maximal participants are still considered. The effectively invited
        // participants are also capped at MAX_ALLOWED_PARTICIPANTS, so excluding these options from the selection
        // would not make sense.
        #[test]
        fn max_is_clamped_at_max_allowed_participants() {
            assert_eq!(
                Some(MAX_ALLOWED_PARTICIPANTS),
                max_participants(
                    &[poll_option(1, MAX_ALLOWED_PARTICIPANTS + 1)],
                    MAX_ALLOWED_PARTICIPANTS
                )
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

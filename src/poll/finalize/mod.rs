use super::{Answer, Attendance, DateSelectionStrategy, Poll, PollOption, PollState};
use crate::database::Repository;
use crate::email::EmailSender;
use crate::event::Event;
use crate::users::{User, UserId};
use crate::UrlPrefix;
use anyhow::Result;
use itertools::{Either, Itertools};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::cmp::min;
use time::OffsetDateTime;

mod scheduling;
pub(crate) use scheduling::*;
mod emails;

async fn finalize(ctx: &mut FinalizeContext) -> Result<()> {
    // not using a transaction here because we're the only ones setting polls to closed.
    if let Some(poll) = ctx.repository.get_current_poll().await? {
        if poll.state(OffsetDateTime::now_utc()) == PollState::PendingClosure {
            try_finalize_poll(ctx, poll).await?;
        }
    }

    Ok(())
}

struct FinalizeContext {
    repository: Box<dyn Repository>,
    email_sender: Box<dyn EmailSender>,
    url_prefix: UrlPrefix<'static>,
}

async fn try_finalize_poll(ctx: &mut FinalizeContext, poll: Poll) -> Result<()> {
    ctx.repository.close_poll(poll.id).await?;

    let result = finalize_poll_dry_run(poll);

    if let FinalizeResult::Success(event, invited, _) = result {
        let event = ctx.repository.add_event(event).await?;
        emails::send_notification_emails(ctx, &event, &invited).await?;
    }

    Ok(())
}

fn finalize_poll_dry_run(poll: Poll) -> FinalizeResult {
    let candidates = get_candidates(&poll);
    if let Some(chosen_option) = choose_option(candidates, &poll) {
        let (invited, not_invited) =
            choose_participants(&chosen_option.answers, poll.max_participants);
        let event = Event::new(&poll, &chosen_option, &invited);
        FinalizeResult::Success(event, invited, not_invited)
    } else {
        FinalizeResult::Failure(poll)
    }
}

#[derive(Debug)]
enum FinalizeResult {
    /// Date selected, some people might not be invited though.
    Success(Event<(), UserId, i64>, Vec<User>, Vec<User>),
    /// No date found because there weren't enough people.
    Failure(Poll),
}

fn get_candidates(poll: &Poll) -> Vec<PollOption> {
    poll.options
        .iter()
        .cloned()
        .filter(|o| !o.has_veto())
        .filter(|o| o.count_yes_answers() >= poll.min_participants)
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

fn max_participants<Id, UserRef>(
    options: &[PollOption<Id, UserRef>],
    max_allowed_participants: usize,
) -> Option<usize> {
    options
        .iter()
        .map(|o| o.count_yes_answers())
        .max()
        .map(|max| min(max, max_allowed_participants))
}

fn choose_participants<Id, UserRef: Clone>(
    answers: &[Answer<Id, UserRef>],
    max_participants: usize,
) -> (Vec<UserRef>, Vec<UserRef>) {
    let (mut accepted, mut rejected): (Vec<_>, Vec<_>) = pre_partition_by_attendance(answers);

    let available = max_participants.saturating_sub(accepted.len());
    if available > 0 {
        rejected.shuffle(&mut thread_rng());
        accepted.extend(rejected.drain(..min(available, rejected.len())));
    }

    (accepted, rejected)
}

fn pre_partition_by_attendance<Id, UserRef: Clone>(
    answers: &[Answer<Id, UserRef>],
) -> (Vec<UserRef>, Vec<UserRef>) {
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
    use crate::poll::AnswerValue;
    use crate::users::UserId;

    mod choose_participants {
        use super::*;

        const MAX_ALLOWED_PARTICIPANTS: usize = 5;

        #[test]
        fn accepted_and_rejected_participants_are_empty_for_empty_answers() {
            let (accepted, rejected) =
                choose_participants::<(), UserId>(&[], MAX_ALLOWED_PARTICIPANTS);
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
            assert_eq!(vec![UserId(1), UserId(2), UserId(3)], accepted);
            assert!(rejected.is_empty());
        }

        // TODO: more tests
    }

    mod max_participants {
        use super::*;

        const MAX_ALLOWED_PARTICIPANTS: usize = 5;

        #[test]
        fn max_is_none_if_options_are_empty() {
            assert!(max_participants::<(), ()>(&[], MAX_ALLOWED_PARTICIPANTS).is_none());
        }

        #[test]
        fn max_is_max_of_all_yes_answers() {
            assert_eq!(
                Some(4),
                max_participants(
                    &[poll_option(0), poll_option(1), poll_option(4)],
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
                    &[poll_option(MAX_ALLOWED_PARTICIPANTS + 1)],
                    MAX_ALLOWED_PARTICIPANTS
                )
            );
        }
    }

    fn poll_option(yes_answers: usize) -> PollOption<(), ()> {
        PollOption {
            id: (),
            starts_at: OffsetDateTime::now_utc(),
            ends_at: OffsetDateTime::now_utc(),
            answers: (0..yes_answers)
                .map(|_| answer(AnswerValue::yes(Attendance::Optional), ()))
                .collect(),
        }
    }

    fn answer<UserRef>(value: AnswerValue, user: UserRef) -> Answer<(), UserRef> {
        Answer {
            value,
            id: (),
            user,
        }
    }
}

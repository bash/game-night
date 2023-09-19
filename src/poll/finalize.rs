use super::{Answer, Attendance, DateSelectionStrategy, Poll, PollOption, PollState};
use crate::database::Repository;
use crate::event::Event;
use crate::users::UserId;
use crate::RocketExt;
use anyhow::Result;
use itertools::{Either, Itertools};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rocket::fairing::{self, Fairing};
use rocket::tokio::time::interval;
use rocket::tokio::{self, select};
use rocket::{warn, Orbit, Rocket, Shutdown};
use std::cmp::min;
use std::time::Duration;
use time::OffsetDateTime;

pub(crate) fn poll_finalizer() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Poll Finalizer", |rocket| {
        Box::pin(async move {
            if let Err(e) = start_finalizer(rocket).await {
                warn!("{:?}", e);
            }
        })
    })
}

async fn start_finalizer(rocket: &Rocket<Orbit>) -> Result<()> {
    tokio::spawn(run_finalizer(rocket.shutdown(), rocket.repository().await?));
    Ok(())
}

async fn run_finalizer(mut shutdown: Shutdown, mut repository: Box<dyn Repository>) {
    const MINUTE: Duration = Duration::from_secs(60);
    let mut interval = interval(5 * MINUTE);
    loop {
        select! {
            _  = &mut shutdown => break,
            _ = interval.tick() => finalize(&mut *repository).await
        }
    }
}

pub(crate) async fn finalize(repository: &mut dyn Repository) {
    if let Err(error) = try_finalize(repository).await {
        warn!("poll finalization failed: {error:?}");
    }
}

pub(crate) async fn try_finalize(repository: &mut dyn Repository) -> Result<()> {
    // not using a transaction here because we're the only ones setting polls to closed.
    if let Some(poll) = repository.get_current_poll().await? {
        if poll.state(OffsetDateTime::now_utc()) == PollState::PendingClosure {
            try_finalize_poll(repository, poll).await?;
        }
    }

    Ok(())
}

async fn try_finalize_poll(repository: &mut dyn Repository, poll: Poll) -> Result<()> {
    repository.close_poll(poll.id).await?;

    let candidates = get_candidates(&poll);

    if let Some(chosen_option) = choose_option(candidates, &poll) {
        let (accepted, _) = choose_participants(&chosen_option.answers, poll.max_participants);
        let event = Event::new(&poll, &chosen_option, &accepted);
        repository.add_event(&event).await?;
        // TODO: send email
    } else {
        // No date found, send email to everyone
    }

    Ok(())
}

fn get_candidates(poll: &Poll) -> Vec<PollOption> {
    poll.options
        .iter()
        .cloned()
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
        .into_iter()
        .map(|o| o.count_yes_answers())
        .max()
        .map(|max| min(max, max_allowed_participants))
}

fn choose_participants(answers: &[Answer], max_participants: usize) -> (Vec<UserId>, Vec<UserId>) {
    let (mut accepted, mut rejected): (Vec<_>, Vec<_>) = pre_partition_by_attendance(answers);

    let available = max_participants.saturating_sub(accepted.len());
    if available > 0 {
        rejected.shuffle(&mut thread_rng());
        accepted.extend(rejected.drain(..min(available, rejected.len())));
    }

    (accepted, rejected)
}

fn pre_partition_by_attendance(answers: &[Answer]) -> (Vec<UserId>, Vec<UserId>) {
    answers
        .into_iter()
        .filter_map(|a| a.yes())
        .partition_map(|a| match a.0 {
            Attendance::Required => Either::Left(a.1),
            Attendance::Optional => Either::Right(a.1),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod max_participants {
        use super::*;
        use crate::poll::AnswerValue;

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

        fn poll_option(yes_answers: usize) -> PollOption<(), ()> {
            PollOption {
                id: (),
                datetime: OffsetDateTime::now_utc(),
                answers: (0..yes_answers)
                    .map(|_| Answer {
                        value: AnswerValue::yes(Attendance::Optional),
                        id: (),
                        user: (),
                    })
                    .collect(),
            }
        }
    }
}

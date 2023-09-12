use super::{Answer, Attendance, DateSelectionStrategy, Poll, PollOption, PollState};
use crate::database::Repository;
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
        let (_, _) = choose_participants(chosen_option.answers, poll.max_participants);
        // TODO: create event
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
        .filter(|o| o.count_participants() >= poll.min_participants)
        .collect()
}

fn choose_option(mut candidates: Vec<PollOption>, poll: &Poll) -> Option<PollOption> {
    use DateSelectionStrategy::*;
    match poll.strategy {
        AtRandom => candidates.choose(&mut thread_rng()).cloned(),
        ToMaximizeParticipants => {
            let max_participants = min(
                poll.max_participants,
                candidates
                    .iter()
                    .map(|o| o.count_participants())
                    .max()
                    .unwrap_or(usize::MAX),
            );
            candidates.retain(|o| (o.count_participants()) >= max_participants);
            candidates.choose(&mut thread_rng()).cloned()
        }
    }
}

fn choose_participants(
    answers: Vec<Answer>,
    max_participants: usize,
) -> (Vec<UserId>, Vec<UserId>) {
    let (mut accepted, mut rejected): (Vec<_>, Vec<_>) = pre_partition_by_attendance(answers);

    let available = max_participants.saturating_sub(accepted.len());
    if available > 0 {
        rejected.shuffle(&mut thread_rng());
        accepted.extend(rejected.drain(..available));
    }

    (accepted, rejected)
}

fn pre_partition_by_attendance(answers: Vec<Answer>) -> (Vec<UserId>, Vec<UserId>) {
    answers
        .into_iter()
        .filter_map(|a| a.yes())
        .partition_map(|a| match a.0 {
            Attendance::Required => Either::Left(a.1),
            Attendance::Optional => Either::Right(a.1),
        })
}

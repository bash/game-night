use super::{Poll, PollState};
use crate::database::Repository;
use crate::RocketExt;
use anyhow::Result;
use rocket::fairing::{self, Fairing};
use rocket::tokio::time::interval;
use rocket::tokio::{self, select};
use rocket::{warn, Orbit, Rocket, Shutdown};
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
    // TODO
    Ok(())
}

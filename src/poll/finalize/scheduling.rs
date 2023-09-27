use super::finalize;
use crate::database::Repository;
use crate::RocketExt;
use anyhow::Result;
use rocket::fairing::{self, Fairing};
use rocket::tokio::time::interval;
use rocket::tokio::{self, select};
use rocket::{warn, Orbit, Rocket, Shutdown};
use std::time::Duration;

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
            _ = interval.tick() => finalize_with_error_handling(&mut *repository).await
        }
    }
}

async fn finalize_with_error_handling(repository: &mut dyn Repository) {
    if let Err(error) = finalize(repository).await {
        warn!("poll finalization failed: {error:?}");
    }
}

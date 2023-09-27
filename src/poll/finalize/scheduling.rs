use super::finalize;
use crate::database::Repository;
use crate::email::EmailSender;
use crate::RocketExt;
use anyhow::{Context as _, Result};
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
    let repository = rocket.repository().await?;
    let email_sender =
        rocket.state::<Box<dyn EmailSender>>().context("email sender not found")?.clone();
    tokio::spawn(run_finalizer(rocket.shutdown(), repository, email_sender));
    Ok(())
}

async fn run_finalizer(
    mut shutdown: Shutdown,
    mut repository: Box<dyn Repository>,
    email_sender: Box<dyn EmailSender>,
) {
    const MINUTE: Duration = Duration::from_secs(60);
    let mut interval = interval(5 * MINUTE);
    loop {
        select! {
            _  = &mut shutdown => break,
            _ = interval.tick() => finalize_with_error_handling(&mut *repository, &*email_sender).await
        }
    }
}

async fn finalize_with_error_handling(
    repository: &mut dyn Repository,
    email_sender: &dyn EmailSender,
) {
    if let Err(error) = finalize(repository, email_sender).await {
        warn!("poll finalization failed: {error:?}");
    }
}

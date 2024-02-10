use super::{finalize, FinalizeContext};
use crate::uri::HasUrlPrefix as _;
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
    let context = FinalizeContext::from_rocket(rocket).await?;
    tokio::spawn(run_finalizer(rocket.shutdown(), context));
    Ok(())
}

impl FinalizeContext {
    async fn from_rocket(rocket: &Rocket<Orbit>) -> Result<Self> {
        Ok(Self {
            repository: rocket.repository().await?,
            email_sender: rocket.email_sender()?,
            url_prefix: rocket.url_prefix()?.to_static(),
        })
    }
}

async fn run_finalizer(mut shutdown: Shutdown, mut context: FinalizeContext) {
    const MINUTE: Duration = Duration::from_secs(60);
    let mut interval = interval(5 * MINUTE);
    loop {
        select! {
            _  = &mut shutdown => break,
            _ = interval.tick() => finalize_with_error_handling(&mut context).await
        }
    }
}

async fn finalize_with_error_handling(context: &mut FinalizeContext) {
    if let Err(error) = finalize(context).await {
        warn!("poll finalization failed: {error:?}");
    }
}

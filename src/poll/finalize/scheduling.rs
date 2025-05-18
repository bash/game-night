use super::{finalize, FinalizeContext};
use crate::services::RocketResolveExt as _;
use anyhow::{Context, Result};
use rocket::fairing::{self, Fairing};
use rocket::tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use rocket::tokio::sync::RwLock;
use rocket::tokio::time::interval;
use rocket::tokio::{self, select};
use rocket::{async_trait, warn, Build, Orbit, Rocket, Shutdown};
use std::time::Duration;

pub(crate) fn poll_finalizer() -> impl Fairing {
    let (tx, rx) = unbounded_channel();
    PollFinalizer {
        rx: RwLock::new(Some(rx)),
        tx,
    }
}

struct PollFinalizer {
    rx: RwLock<Option<UnboundedReceiver<()>>>,
    tx: UnboundedSender<()>,
}

#[async_trait]
impl Fairing for PollFinalizer {
    fn info(&self) -> fairing::Info {
        fairing::Info {
            name: "Poll Finalizer",
            kind: fairing::Kind::Liftoff | fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        Ok(rocket.manage(NudgeFinalizer(self.tx.clone())))
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        if let Err(e) = start_finalizer(rocket, &self.rx).await {
            warn!("{:?}", e);
        }
    }
}

#[derive(Debug)]
pub(crate) struct NudgeFinalizer(UnboundedSender<()>);

impl NudgeFinalizer {
    pub(crate) fn nudge(&self) {
        _ = self.0.send(());
    }
}

async fn start_finalizer(
    rocket: &Rocket<Orbit>,
    rx: &RwLock<Option<UnboundedReceiver<()>>>,
) -> Result<()> {
    let rx = rx.write().await.take().context("receiver consumed twice")?;
    let context = rocket.resolve().await?;
    tokio::spawn(run_finalizer(rx, rocket.shutdown(), context));
    Ok(())
}

async fn run_finalizer(
    mut rx: UnboundedReceiver<()>,
    mut shutdown: Shutdown,
    mut context: FinalizeContext,
) {
    const MINUTE: Duration = Duration::from_secs(60);
    let mut interval = interval(5 * MINUTE);
    loop {
        select! {
            _  = &mut shutdown => break,
            _ = rx.recv() => finalize_with_error_handling(&mut context).await,
            _ = interval.tick() => finalize_with_error_handling(&mut context).await
        }
    }
}

async fn finalize_with_error_handling(context: &mut FinalizeContext) {
    if let Err(error) = finalize(context).await {
        warn!("poll finalization failed: {error:#?}");
    }
}

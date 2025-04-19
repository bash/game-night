use crate::database::Repository;
use crate::services::RocketResolveExt as _;
use anyhow::Result;
use rocket::fairing::{self, Fairing};
use rocket::tokio::time::interval;
use rocket::tokio::{self, select};
use rocket::{info, warn, Orbit, Rocket, Shutdown};
use std::time::Duration;

pub(crate) fn database_pruning() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Database Pruning", |rocket| {
        Box::pin(start_pruning_with_error_handling(rocket))
    })
}

async fn start_pruning_with_error_handling(rocket: &Rocket<Orbit>) {
    if let Err(error) = start_pruning(rocket).await {
        warn!("failed to start database pruning: {error:?}");
    }
}

async fn start_pruning(rocket: &Rocket<Orbit>) -> Result<()> {
    let repository = rocket.resolve().await?;
    tokio::spawn(run_pruning(repository, rocket.shutdown()));
    Ok(())
}

async fn run_pruning(mut repository: Box<dyn Repository>, mut shutdown: Shutdown) {
    const HOUR: Duration = Duration::from_secs(60 * 60);
    let mut interval = interval(24 * HOUR);
    loop {
        select! {
            _  = &mut shutdown => break,
            _ = interval.tick() => prune_with_error_handling(&mut *repository).await
        }
    }
}

async fn prune_with_error_handling(repository: &mut dyn Repository) {
    match repository.prune().await {
        Ok(deleted) if deleted >= 1 => info!("🌳 Database pruning removed {deleted} rows"),
        Ok(_) => {}
        Err(error) => warn!("🌳 Database pruning failed: {error:?}"),
    };
}

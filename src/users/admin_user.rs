use crate::invitation;
use anyhow::{Context as _, Result};
use rocket::fairing::{self, Fairing};
use rocket::{error, Orbit, Rocket};

pub(crate) fn invite_admin_user_fairing() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Invite Admin User", |rocket| {
        Box::pin(async move {
            if let Err(e) = try_invite_admin_user(rocket).await {
                error!("{:?}", e);
            }
        })
    })
}

async fn try_invite_admin_user(rocket: &Rocket<Orbit>) -> Result<()> {
    use crate::services::RocketResolveExt as _;
    let mut repository: Box<dyn crate::database::Repository> = rocket.resolve().await?;
    invitation::invite_admin_user(&mut *repository)
        .await
        .context("failed to invite admin user")
}

use crate::services::{Resolve, ResolveContext};
use anyhow::{Context as _, Result};
use rocket::figment::Figment;

#[derive(Debug, Clone)]
pub(super) struct VapidContact(pub(super) String);

impl VapidContact {
    pub(super) fn from_figment(figment: &Figment) -> Result<Self> {
        Ok(VapidContact(
            figment.focus("web_push").extract_inner("vapid_contact")?,
        ))
    }
}

impl Resolve for VapidContact {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        ctx.rocket()
            .state()
            .context("VAPID contact not registered")
            .cloned()
    }
}

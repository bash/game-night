use crate::services::{Resolve, ResolveContext};
use anyhow::{Context as _, Result};

pub(crate) type HttpClient = reqwest::Client;

impl Resolve for HttpClient {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        ctx.rocket()
            .state()
            .context("http client not registered")
            .cloned()
    }
}

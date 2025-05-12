use crate::impl_resolve_for_state;
use anyhow::{Context as _, Result};
use lettre::message::MultiPart;
use rocket::tokio::fs::read_to_string;
use std::env;

pub(crate) trait EmailTemplate {
    fn render(&self) -> askama::Result<EmailBody>;
}

#[derive(Debug, Clone)]
pub(crate) struct EmailTemplateContext {
    pub(crate) css: String,
}

impl_resolve_for_state! { EmailTemplateContext: "email template context" }

#[derive(Debug, Clone)]
pub(crate) struct EmailBody {
    pub(crate) plain: String,
    pub(crate) html: String,
}

impl EmailTemplateContext {
    pub(crate) async fn new() -> Result<Self> {
        let mut css_file_path = env::current_exe()?
            .canonicalize()?
            .parent()
            .context("no parent directory")?
            .to_owned();
        css_file_path.push("email.css");
        Ok(Self {
            css: read_to_string(css_file_path)
                .await
                .context("email.css is missing")?,
        })
    }
}

impl From<EmailBody> for MultiPart {
    fn from(value: EmailBody) -> Self {
        MultiPart::alternative_plain_html(value.plain, value.html)
    }
}

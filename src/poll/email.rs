use super::{Open, Poll};
use crate::email::{EmailMessage, EmailSender};
use crate::fmt::LongEventTitle;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Absolute;
use rocket::response::content::RawHtml;
use rocket::response::Debug;
use rocket::{get, State};
use serde::Serialize;

#[get("/poll/email-preview")]
pub(super) fn poll_email_preview(
    user: User,
    poll: Open<Poll>,
    email_sender: &State<Box<dyn EmailSender>>,
    uri_builder: UriBuilder,
) -> Result<RawHtml<String>, Debug<Error>> {
    let email = PollEmail {
        name: user.name,
        poll: poll.into_inner(),
        poll_uri: uri!(uri_builder, super::open::open_poll_page),
        skip_poll_uri: uri!(uri_builder, super::skip::skip_poll),
        manage_subscription_url: uri!(uri_builder, crate::register::profile),
    };
    let body = email_sender.preview(&email)?;
    Ok(RawHtml(body.html))
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail<'a> {
    pub(super) name: String,
    pub(super) poll: Poll,
    pub(super) poll_uri: Absolute<'a>,
    pub(super) skip_poll_uri: Absolute<'a>,
    pub(super) manage_subscription_url: Absolute<'a>,
}

impl EmailMessage for PollEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "Pick a Date for {title}",
            title = LongEventTitle(&self.poll.event.title)
        )
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }
}

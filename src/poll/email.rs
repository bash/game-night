use crate::email::{EmailMessage, EmailSender};
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Absolute;
use rocket::response::content::RawHtml;
use rocket::response::Debug;
use rocket::{get, State};
use serde::Serialize;
use time::OffsetDateTime;

use super::{Open, Poll};

#[get("/poll/email-preview")]
pub(super) fn poll_email_preview(
    user: User,
    poll: Open<Poll>,
    email_sender: &State<Box<dyn EmailSender>>,
    uri_builder: UriBuilder,
) -> Result<RawHtml<String>, Debug<Error>> {
    let email = PollEmail {
        name: user.name,
        poll_closes_at: poll.open_until,
        poll_url: uri!(uri_builder, super::open::open_poll_page),
        manage_subscription_url: uri!(uri_builder, crate::register::profile),
    };
    let body = email_sender.preview(&email)?;
    Ok(RawHtml(body.html))
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail<'a> {
    pub(super) name: String,
    #[serde(with = "time::serde::iso8601")]
    pub(super) poll_closes_at: OffsetDateTime,
    pub(super) poll_url: Absolute<'a>,
    pub(super) manage_subscription_url: Absolute<'a>,
}

impl EmailMessage for PollEmail<'_> {
    fn subject(&self) -> String {
        "Are You Ready for a Game Night?".to_owned()
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }
}

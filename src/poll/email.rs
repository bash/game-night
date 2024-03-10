use super::{Open, Poll};
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
        poll_url: uri!(uri_builder, super::open::open_poll_page),
        manage_subscription_url: uri!(uri_builder, crate::register::profile),
    };
    let body = email_sender.preview(&email)?;
    Ok(RawHtml(body.html))
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail<'a, Id, UserRef, LocationRef> {
    pub(super) name: String,
    pub(super) poll: Poll<Id, UserRef, LocationRef>,
    pub(super) poll_url: Absolute<'a>,
    pub(super) manage_subscription_url: Absolute<'a>,
}

impl<Id, UserRef, LocationRef> EmailMessage for PollEmail<'_, Id, UserRef, LocationRef>
where
    Id: Serialize + Send + Sync,
    UserRef: Serialize + Send + Sync,
    LocationRef: Serialize + Send + Sync,
{
    fn subject(&self) -> String {
        "Are You Ready for a Game Night?".to_owned()
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }
}

use super::Poll;
use crate::email::{EmailMessage, EmailSender};
use crate::event::EventsQuery;
use crate::fmt::LongEventTitle;
use crate::result::HttpResult;
use crate::uri::UriBuilder;
use crate::users::User;
use crate::{responder, uri};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::response::content::RawHtml;
use rocket::{get, State};
use serde::Serialize;

#[get("/event/<id>/poll/email-preview?<txt>")]
pub(super) async fn poll_email_preview(
    id: i64,
    user: User,
    txt: bool,
    mut events: EventsQuery,
    email_sender: &State<Box<dyn EmailSender>>,
    uri_builder: UriBuilder<'_>,
) -> HttpResult<PlainOrHtml> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::NotFound.into());
    };
    let event_id = poll.event.id;
    let email = PollEmail {
        name: user.name,
        poll,
        poll_uri: uri!(uri_builder, crate::event::event_page(id = event_id)),
        skip_poll_uri: uri!(uri_builder, super::skip::skip_poll(id = event_id)),
        manage_subscription_url: uri!(uri_builder, crate::register::profile),
    };
    let body = email_sender.preview(&email)?;
    if txt {
        Ok(body.plain.into())
    } else {
        Ok(RawHtml(body.html).into())
    }
}

responder! {
    pub(crate) enum PlainOrHtml {
        Plain(String),
        Html(RawHtml<String>),
    }
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

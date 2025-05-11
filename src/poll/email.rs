use super::{Poll, PollOption};
use crate::decorations::Random;
use crate::email::{EmailMessage, EmailTemplate, EmailTemplateContext};
use crate::event::EventsQuery;
use crate::fmt::LongEventTitle;
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::uri::UriBuilder;
use crate::users::User;
use crate::{email_template, responder, uri};
use itertools::Itertools;
use rocket::get;
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::response::content::RawHtml;

#[get("/event/<id>/poll/email-preview?<txt>")]
pub(super) async fn poll_email_preview(
    id: i64,
    user: User,
    txt: bool,
    mut events: EventsQuery,
    uri_builder: UriBuilder,
    email_ctx: EmailTemplateContext,
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
        random: Random::default(),
        ctx: email_ctx,
    };
    let body = email.render()?;
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

email_template! {
    #[template(html_path = "emails/poll.html", txt_path = "emails/poll.txt")]
    #[derive(Debug)]
    pub(super) struct PollEmail<'a> {
        pub(super) name: String,
        pub(super) poll: Poll,
        pub(super) poll_uri: Absolute<'a>,
        pub(super) skip_poll_uri: Absolute<'a>,
        pub(super) manage_subscription_url: Absolute<'a>,
        pub(super) random: Random,
        pub(super) ctx: EmailTemplateContext,
    }
}

impl PollEmail<'_> {
    fn sorted_options(&self) -> Vec<PollOption> {
        self.poll
            .options
            .iter()
            .sorted_by_key(|o| o.starts_at.0)
            .cloned()
            .collect()
    }
}

impl EmailMessage for PollEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "Pick a Date for {title}",
            title = LongEventTitle(&self.poll.event.title)
        )
    }
}

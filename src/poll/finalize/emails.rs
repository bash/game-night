use crate::email::{EmailMessage, EmailSender};
use crate::event::Event;
use crate::fmt::LongEventTitle;
use crate::play::rocket_uri_macro_play_page;
use crate::uri::{HasUriBuilder as _, UriBuilder};
use crate::users::User;
use crate::{uri, RocketExt as _};
use anyhow::{Error, Result};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, SinglePart};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Orbit, Request, Rocket};
use serde::Serialize;
use time::format_description::FormatItem;
use time::macros::format_description;

pub(super) async fn send_notification_emails(
    sender: &EventEmailSender,
    event: &Event,
    invited: &[User],
    missed: &[User],
) -> Result<()> {
    for user in invited {
        sender.send(event, user).await?;
    }
    for user in missed {
        sender.send_missed(event, user).await?;
    }
    Ok(())
}

pub(crate) struct EventEmailSender {
    email_sender: Box<dyn EmailSender>,
    uri_builder: UriBuilder<'static>,
}

impl EventEmailSender {
    pub(crate) async fn from_rocket(rocket: &Rocket<Orbit>) -> Result<Self> {
        Ok(Self {
            email_sender: rocket.email_sender()?,
            uri_builder: rocket.uri_builder().await?.into_static(),
        })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for EventEmailSender {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match EventEmailSender::from_rocket(request.rocket()).await {
            Ok(sender) => Outcome::Success(sender),
            Err(e) => Outcome::Error((Status::InternalServerError, e)),
        }
    }
}

impl EventEmailSender {
    pub(crate) async fn send(&self, event: &Event, user: &User) -> Result<()> {
        let event_url =
            uri!(auto_login(user, event.ends_at); self.uri_builder, play_page()).await?;
        let ics_file = crate::play::to_calendar(event, &self.uri_builder)?.to_string();
        let email = InvitedEmail {
            event,
            event_url,
            name: &user.name,
            ics_file,
        };
        self.email_sender.send(user.mailbox()?, &email).await?;
        Ok(())
    }

    pub(crate) async fn send_missed(&self, event: &Event, user: &User) -> Result<()> {
        let event_url =
            uri!(auto_login(user, event.ends_at); self.uri_builder, play_page()).await?;
        let email = MissedEmail {
            event,
            event_url,
            name: &user.name,
        };
        self.email_sender.send(user.mailbox()?, &email).await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct InvitedEmail<'a> {
    event: &'a Event,
    name: &'a str,
    event_url: Absolute<'a>,
    ics_file: String,
}

impl<'a> EmailMessage for InvitedEmail<'a> {
    fn subject(&self) -> String {
        const FORMAT: &[FormatItem<'_>] =
            format_description!("[day padding:none]. [month repr:long]");
        format!(
            "You're invited to {title} on {date}!",
            date = self.event.starts_at.format(FORMAT).unwrap(),
            title = LongEventTitle(&self.event.title),
        )
    }

    fn template_name(&self) -> String {
        "event/invited".to_string()
    }

    fn attachments(&self) -> Result<Vec<SinglePart>> {
        let ics_attachment = Attachment::new("game-night.ics".to_string())
            .body(self.ics_file.clone(), ContentType::parse("text/calendar")?);
        Ok(vec![ics_attachment])
    }
}

#[derive(Debug, Serialize)]
struct MissedEmail<'a> {
    event: &'a Event,
    name: &'a str,
    event_url: Absolute<'a>,
}

impl<'a> EmailMessage for MissedEmail<'a> {
    fn subject(&self) -> String {
        const FORMAT: &[FormatItem<'_>] =
            format_description!("[day padding:none]. [month repr:long]");
        format!(
            "{title} is happening on {date}!",
            date = self.event.starts_at.format(FORMAT).unwrap(),
            title = LongEventTitle(&self.event.title),
        )
    }

    fn template_name(&self) -> String {
        "event/missed".to_string()
    }
}

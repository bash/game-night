use crate::email::EmailMessage;
use crate::event::{Event, EventEmailSender as DynEventEmailSender, Ics};
use crate::fmt::LongEventTitle;
use crate::uri::{HasUriBuilder as _, UriBuilder};
use crate::users::User;
use crate::{uri, RocketExt as _};
use anyhow::{Error, Result};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, SinglePart};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Orbit, Request, Rocket};
use serde::Serialize;
use time::format_description::FormatItem;
use time::macros::format_description;

pub(super) async fn send_notification_emails(
    sender: &mut EventEmailSender,
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

// TODO: rename this to something more suiting?
// Maybe FinalizePollEmailSender?
pub(crate) struct EventEmailSender {
    email_sender: Box<dyn DynEventEmailSender>,
    uri_builder: UriBuilder<'static>,
}

impl EventEmailSender {
    pub(crate) async fn from_rocket(rocket: &Rocket<Orbit>) -> Result<Self> {
        Ok(Self {
            email_sender: rocket.event_email_sender().await?,
            uri_builder: rocket.uri_builder().await?.into_static(),
        })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for EventEmailSender {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        EventEmailSender::from_rocket(request.rocket())
            .await
            .or_error(Status::InternalServerError)
    }
}

impl EventEmailSender {
    pub(crate) async fn send(&mut self, event: &Event, user: &User) -> Result<()> {
        let event_url =
            uri!(auto_login(user, event.estimated_ends_at()); self.uri_builder, crate::event::event_page(id = event.id))
                .await?;
        let ics_file = Ics::from_event(event, &self.uri_builder)?.0;
        let email = InvitedEmail {
            event,
            event_url,
            name: &user.name,
            ics_file,
        };
        self.email_sender.send(event, user, &email).await?;
        Ok(())
    }

    pub(crate) async fn send_missed(&mut self, event: &Event, user: &User) -> Result<()> {
        let event_url =
            uri!(auto_login(user, event.estimated_ends_at()); self.uri_builder, crate::event::event_page(id = event.id))
                .await?;
        let email = MissedEmail {
            event,
            event_url,
            name: &user.name,
        };
        self.email_sender.send(event, user, &email).await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct InvitedEmail<'a> {
    event: &'a Event,
    name: &'a str,
    event_url: Absolute<'a>,
    #[serde(skip)]
    ics_file: String,
}

impl EmailMessage for InvitedEmail<'_> {
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

impl EmailMessage for MissedEmail<'_> {
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

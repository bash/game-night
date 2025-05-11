use crate::decorations::Random;
use crate::email::{EmailMessage, EmailTemplateContext};
use crate::event::{Event, EventEmailSender as DynEventEmailSender, Ics};
use crate::fmt::LongEventTitle;
use crate::push::{InvitedNotification, MissedNotification, PushSender};
use crate::template::prelude::*;
use crate::uri::UriBuilder;
use crate::users::User;
use crate::{auto_resolve, email_template, uri};
use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, SinglePart};
use rocket::http::uri::Absolute;
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
// Maybe FinalizePollNotificationSender?
auto_resolve! {
    pub(crate) struct EventEmailSender {
        email_sender: Box<dyn DynEventEmailSender + 'static>,
        uri_builder: UriBuilder,
        push_sender: PushSender,
        ctx: EmailTemplateContext,
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
            random: Random::default(),
            ctx: self.ctx.clone(),
        };
        self.email_sender.send(event, user, &email).await?;
        let notification = InvitedNotification {
            event,
            random: Random::default(),
        };
        self.push_sender
            .send_templated(&notification, user.id)
            .await?;
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
            random: Random::default(),
            ctx: self.ctx.clone(),
        };
        self.email_sender.send(event, user, &email).await?;
        let notification = MissedNotification { event };
        self.push_sender
            .send_templated(&notification, user.id)
            .await?;
        Ok(())
    }
}

email_template! {
    #[template(html_path = "emails/event/invited.html", txt_path = "emails/event/invited.txt")]
    #[derive(Debug)]
    struct InvitedEmail<'a> {
        event: &'a Event,
        name: &'a str,
        event_url: Absolute<'a>,
        ics_file: String,
        random: Random,
        ctx: EmailTemplateContext,
    }
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

    fn attachments(&self) -> Result<Vec<SinglePart>> {
        let ics_attachment = Attachment::new("game-night.ics".to_string())
            .body(self.ics_file.clone(), ContentType::parse("text/calendar")?);
        Ok(vec![ics_attachment])
    }
}

email_template! {
    #[template(html_path = "emails/event/missed.html", txt_path = "emails/event/missed.txt")]
    #[derive(Debug)]
    struct MissedEmail<'a> {
        event: &'a Event,
        name: &'a str,
        event_url: Absolute<'a>,
        random: Random,
        ctx: EmailTemplateContext,
    }
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
}

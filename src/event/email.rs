use std::fmt;

use crate::{
    database::{EventEmailsRepository, Materialized},
    default,
    email::{EmailMessage, EmailMessageOptions, EmailSender, MessageId},
    event::Event,
    users::{User, UserId},
    RocketExt,
};
use anyhow::{Error, Result};
use rand::{thread_rng, Rng};
use rocket::{
    async_trait,
    http::Status,
    outcome::IntoOutcome,
    request::{FromRequest, Outcome},
    Request,
};

use super::{EventLifecycle, Planned};

pub(crate) fn create_event_email_sender<L: EventLifecycle>(
    repository: Box<dyn EventEmailsRepository>,
    sender: Box<dyn EmailSender>,
) -> Box<dyn EventEmailSender<L>> {
    Box::new(EventEmailSenderImpl { repository, sender })
}

/// An email sender that sends out emails relating
/// to an event / poll.
#[async_trait]
pub(crate) trait EventEmailSender<L: EventLifecycle = Planned>: fmt::Debug + Send {
    async fn send(
        &mut self,
        event: &Event<Materialized, L>,
        recipient: &User,
        email: &dyn EmailMessage,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub(crate) struct EventEmail {
    pub(crate) event: i64,
    pub(crate) user: UserId,
    pub(crate) message_id: MessageId,
    pub(crate) subject: String,
}

#[async_trait]
impl<L: EventLifecycle, 'r> FromRequest<'r> for Box<dyn EventEmailSender<L>> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .rocket()
            .event_email_sender()
            .await
            .or_error(Status::InternalServerError)
    }
}

#[derive(Debug)]
struct EventEmailSenderImpl {
    repository: Box<dyn EventEmailsRepository>,
    sender: Box<dyn EmailSender>,
}

#[async_trait]
impl<L: EventLifecycle> EventEmailSender<L> for EventEmailSenderImpl {
    async fn send(
        &mut self,
        event: &Event<Materialized, L>,
        recipient: &User,
        email: &dyn EmailMessage,
    ) -> Result<()> {
        let in_reply_to = self.get_in_reply_to(event, recipient).await?;
        let message_id: MessageId = thread_rng().gen();

        let options = EmailMessageOptions {
            message_id: Some(message_id.clone()),
            in_reply_to,
            ..default()
        };
        let mailbox = recipient.mailbox()?;
        self.sender.send(mailbox, email, options).await?;

        let event_email = EventEmail {
            event: event.id,
            user: recipient.id,
            message_id,
            subject: email.subject(),
        };
        self.repository.add_event_email(event_email).await?;

        Ok(())
    }
}

impl EventEmailSenderImpl {
    async fn get_in_reply_to<L: EventLifecycle>(
        &mut self,
        event: &Event<Materialized, L>,
        user: &User,
    ) -> Result<Option<MessageId>> {
        self.repository.get_last_message_id(event.id, user.id).await
    }
}

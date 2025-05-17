use crate::{
    database::{EventEmailsRepository, Materialized},
    email::{EmailMessage, EmailMessageOptions, EmailSender, MessageId},
    event::Event,
    impl_from_request_for_service,
    services::{Resolve, ResolveContext},
    users::{User, UserId, UserMailboxExt as _},
};
use anyhow::Result;
use rand::{rng, Rng};
use rocket::async_trait;
use std::fmt;

use super::{EventLifecycle, Planned};

impl<L: EventLifecycle> Resolve for Box<dyn EventEmailSender<L>> {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        let sender = ctx.resolve::<Box<dyn EmailSender>>().await?;
        Ok(Box::new(EventEmailSenderImpl {
            repository: ctx.resolve().await?,
            sender,
        }) as Box<dyn EventEmailSender<L>>)
    }
}

impl_from_request_for_service!(<L: EventLifecycle> Box<dyn EventEmailSender<L>>);

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
        let message_id: MessageId = rng().random();

        let options = EmailMessageOptions {
            message_id: Some(message_id.clone()),
            in_reply_to,
        };
        let mailbox = recipient.mailbox()?;
        self.sender.send(mailbox, email, options).await?;

        let event_email = EventEmail {
            event: get_primary_id(event),
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
        self.repository
            .get_last_message_id(get_primary_id(event), user.id)
            .await
    }
}

fn get_primary_id<L: EventLifecycle>(event: &Event<Materialized, L>) -> i64 {
    event.parent_id.unwrap_or(event.id)
}

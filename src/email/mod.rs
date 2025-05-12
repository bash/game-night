use crate::impl_resolve_for_state;
use anyhow::{Context as _, Result};
use dyn_clone::DynClone;
use headers::MessageBuilderExt;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::Message;
#[cfg(unix)]
use outbox::Outbox;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::{async_trait, error, Build, Rocket};
use serde::Deserialize;
use std::fmt;

mod headers;
mod message_id;
pub(crate) use message_id::*;
mod render;
pub(crate) use render::*;

pub(crate) fn email_sender_fairing() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Email Sender", |rocket| {
        Box::pin(async {
            match fairing_impl(&rocket).await {
                Ok((sender, context)) => Ok(rocket.manage(sender).manage(context)),
                Err(error) => {
                    error!("failed to initialize email sender:\n{:?}", error);
                    Err(rocket)
                }
            }
        })
    })
}

async fn fairing_impl(
    rocket: &Rocket<Build>,
) -> Result<(Box<dyn EmailSender>, EmailTemplateContext)> {
    let sender = EmailSenderImpl::from_figment(rocket.figment()).await?;
    let context = EmailTemplateContext::new().await?;
    Ok((Box::new(sender), context))
}

#[async_trait]
pub(crate) trait EmailSender: Send + Sync + fmt::Debug + DynClone {
    async fn send(
        &self,
        recipient: Mailbox,
        email: &dyn EmailMessage,
        options: EmailMessageOptions,
    ) -> Result<()>;
}

dyn_clone::clone_trait_object!(EmailSender);

impl_resolve_for_state!(Box<dyn EmailSender>: "email sender");

pub(crate) trait EmailMessage: Send + Sync + EmailTemplate {
    fn subject(&self) -> String;

    fn reply_to(&self) -> Option<Mailbox> {
        None
    }

    fn attachments(&self) -> Result<Vec<SinglePart>> {
        Ok(Vec::default())
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct EmailMessageOptions {
    pub(crate) message_id: Option<MessageId>,
    pub(crate) in_reply_to: Option<MessageId>,
}

#[derive(Debug, Clone)]
pub(crate) struct EmailSenderImpl {
    sender: Mailbox,
    reply_to: Option<Mailbox>,
    #[cfg(unix)]
    outbox: Outbox,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(
        &self,
        recipient: Mailbox,
        email: &dyn EmailMessage,
        options: EmailMessageOptions,
    ) -> Result<()> {
        let body = email.render()?;
        let multipart = email
            .attachments()?
            .into_iter()
            .fold(MultiPart::from(body), |m, s| m.singlepart(s));
        let mut builder = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(email.subject())
            .auto_generated();
        if let Some(message_id) = options.message_id {
            // Careful if you think about refactoring this.
            // `builder.message_id(None)` generates a random message id
            // whereas not calling message_id causes no message id to be generated.
            builder = builder.message_id(Some(message_id.to_string()));
        }
        if let Some(message_id) = options.in_reply_to {
            builder = builder
                .in_reply_to(message_id.to_string())
                .references(message_id.to_string());
        }
        if let Some(reply_to) = email.reply_to().or_else(|| self.reply_to.clone()) {
            builder = builder.reply_to(reply_to);
        }
        let message = builder
            .multipart(multipart)
            .context("failed to create email message")?;

        #[cfg(unix)]
        self.outbox.queue(message.formatted()).await?;
        #[cfg(not(unix))]
        println!("{}", String::from_utf8(message.formatted())?);

        Ok(())
    }
}

impl EmailSenderImpl {
    pub(crate) async fn from_figment(figment: &Figment) -> Result<Self> {
        let config: EmailSenderConfig = figment
            .extract_inner("email")
            .context("failed to read email sender configuration")?;
        #[cfg(unix)]
        let outbox =
            Outbox::new_for_path(config.outbox_socket).context("failed to initialize outbox")?;
        Ok(Self {
            sender: config.sender,
            reply_to: config.reply_to,
            #[cfg(unix)]
            outbox,
        })
    }
}

#[derive(Debug, Deserialize)]
struct EmailSenderConfig {
    sender: Mailbox,
    reply_to: Option<Mailbox>,
    #[cfg(unix)]
    outbox_socket: String,
}

use anyhow::{Context as _, Result};
use dyn_clone::DynClone;
use headers::MessageBuilderExt;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::Message;
#[cfg(unix)]
use outbox::Outbox;
use render::EmailRenderer;
use rocket::async_trait;
use rocket::figment::value::magic::RelativePathBuf;
use rocket::figment::Figment;
use rocket_dyn_templates::tera::Context;
use serde::{Deserialize, Serialize};

mod headers;
mod render;
pub(crate) use render::*;

#[async_trait]
pub(crate) trait EmailSender: Send + Sync + DynClone {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()>;

    fn preview(&self, email: &dyn EmailMessage) -> Result<EmailBody>;
}

dyn_clone::clone_trait_object!(EmailSender);

pub(crate) trait EmailMessage: Send + Sync + EmailMessageContext {
    fn subject(&self) -> String;

    fn template_name(&self) -> String;

    fn attachments(&self) -> Result<Vec<SinglePart>> {
        Ok(Vec::default())
    }
}

pub(crate) trait EmailMessageContext {
    fn template_context(&self) -> Result<Context>;
}

impl<T> EmailMessageContext for T
where
    T: Serialize,
{
    fn template_context(&self) -> Result<Context> {
        Context::from_serialize(self).map_err(Into::into)
    }
}

#[derive(Clone)]
pub(crate) struct EmailSenderImpl {
    sender: Mailbox,
    reply_to: Option<Mailbox>,
    #[cfg(unix)]
    outbox: Outbox,
    renderer: EmailRenderer,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()> {
        let body = self.renderer.render(email)?;
        let multipart = email
            .attachments()?
            .into_iter()
            .fold(MultiPart::from(body), |m, s| m.singlepart(s));
        let mut builder = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(email.subject())
            .auto_generated();
        if let Some(reply_to) = &self.reply_to {
            builder = builder.reply_to(reply_to.clone());
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

    fn preview(&self, email: &dyn EmailMessage) -> Result<EmailBody> {
        self.renderer.render(email)
    }
}

impl EmailSenderImpl {
    pub(crate) async fn from_figment(figment: &Figment) -> Result<Self> {
        let config: EmailSenderConfig = figment
            .extract_inner("email")
            .context("failed to read email sender configuration")?;
        let template_dir = config.template_dir.relative();
        #[cfg(unix)]
        let outbox_bus = config.outbox_bus.unwrap_or(OutboxBus::System);
        #[cfg(unix)]
        let outbox = outbox_bus
            .to_outbox()
            .await
            .context("failed to initialize outbox")?;

        Ok(Self {
            sender: config.sender,
            reply_to: config.reply_to,
            #[cfg(unix)]
            outbox,
            renderer: EmailRenderer::from_template_dir(&template_dir).await?,
        })
    }
}

#[cfg(unix)]
#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutboxBus {
    System,
    Session,
}

#[cfg(unix)]
impl OutboxBus {
    async fn to_outbox(self) -> Result<Outbox> {
        match self {
            OutboxBus::Session => Ok(Outbox::session().await?),
            OutboxBus::System => Ok(Outbox::system().await?),
        }
    }
}

#[derive(Debug, Deserialize)]
struct EmailSenderConfig {
    sender: Mailbox,
    reply_to: Option<Mailbox>,
    template_dir: RelativePathBuf,
    #[cfg(unix)]
    outbox_bus: Option<OutboxBus>,
}

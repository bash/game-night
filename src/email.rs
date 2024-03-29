use crate::decorations::{Closings, Greetings, Hearts, SkinToneModifiers};
use anyhow::{anyhow, Context as _, Result};
use dyn_clone::DynClone;
use lettre::message::header::{Header, HeaderName, HeaderValue};
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::Message;
#[cfg(unix)]
use outbox::Outbox;
use rand::{thread_rng, Rng as _};
use rocket::async_trait;
use rocket::figment::value::magic::RelativePathBuf;
use rocket::figment::Figment;
use rocket::tokio::fs::read_to_string;
use rocket_dyn_templates::tera::{Context, Tera};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[async_trait]
pub(crate) trait EmailSender: Send + Sync + DynClone {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()>;

    fn preview(&self, email: &dyn EmailMessage) -> Result<EmailBody>;
}

pub(crate) struct EmailBody {
    pub(crate) plain: String,
    pub(crate) html: String,
}

impl From<EmailBody> for MultiPart {
    fn from(value: EmailBody) -> Self {
        MultiPart::alternative_plain_html(value.plain, value.html)
    }
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
    #[cfg(unix)]
    outbox: Outbox,
    css: String,
    tera: Tera,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()> {
        let multipart = email
            .attachments()?
            .into_iter()
            .fold(MultiPart::from(self.render_email_body(email)?), |m, s| {
                m.singlepart(s)
            });
        let message = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(email.subject())
            .header(AutoSubmitted::AutoGenerated)
            .header(XAutoResponseSuppress::All)
            .multipart(multipart)
            .context("failed to create email message")?;

        #[cfg(unix)]
        self.outbox.queue(message.formatted()).await?;
        #[cfg(not(unix))]
        println!("{}", String::from_utf8(message.formatted())?);

        Ok(())
    }

    fn preview(&self, email: &dyn EmailMessage) -> Result<EmailBody> {
        self.render_email_body(email)
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
        let mut css_file_path = template_dir.clone();
        css_file_path.push("email.css");

        Ok(Self {
            sender: config.sender,
            #[cfg(unix)]
            outbox,
            tera: create_tera(&template_dir)?,
            css: read_to_string(css_file_path)
                .await
                .context("email.css is missing")?,
        })
    }

    fn render_email_body(&self, email: &dyn EmailMessage) -> Result<EmailBody> {
        let template_name = email.template_name();
        let mut template_context = email.template_context()?;
        let mut rng = thread_rng();
        template_context.insert("greeting", rng.sample(Greetings));
        template_context.insert("closing", rng.sample(Closings));
        template_context.insert("skin_tone", rng.sample(SkinToneModifiers));
        template_context.insert("heart", rng.sample(Hearts));
        template_context.insert("css", &self.css);
        let html_template_name = format!("{}.html.tera", &template_name);
        let text_template_name = format!("{}.txt.tera", &template_name);

        Ok(EmailBody {
            plain: self
                .tera
                .render(&text_template_name, &template_context)
                .context("failed to render tera template")?,
            html: self
                .tera
                .render(&html_template_name, &template_context)
                .context("failed to render tera template")?,
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

fn create_tera(template_dir: &Path) -> Result<Tera> {
    let templates = template_dir.join("**.tera");
    let templates = templates
        .to_str()
        .context("template dir is not valid utf-8")?;
    let mut tera = Tera::new(templates).context("failed to initialize Tera")?;
    tera.build_inheritance_chains()
        .context("failed to build tera's inheritance chain")?;
    crate::template::register_custom_functions(&mut tera);
    Ok(tera)
}

#[derive(Debug, Deserialize)]
struct EmailSenderConfig {
    sender: Mailbox,
    template_dir: RelativePathBuf,
    #[cfg(unix)]
    outbox_bus: Option<OutboxBus>,
}

#[derive(Debug, Copy, Clone)]
enum AutoSubmitted {
    /// Indicates that a message was generated by an automatic process, and is not a direct response to another message.
    /// Automatic responses should not be issued to messages with this header. See <https://www.rfc-editor.org/rfc/rfc3834#section-2>.
    AutoGenerated,
}

impl Header for AutoSubmitted {
    fn name() -> lettre::message::header::HeaderName {
        HeaderName::new_from_ascii_str("Auto-Submitted")
    }

    fn parse(_: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err(anyhow!("Not supported").into())
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        HeaderValue::new(
            Self::name(),
            match self {
                Self::AutoGenerated => "auto-generated".to_owned(),
            },
        )
    }
}

#[derive(Debug, Copy, Clone)]
enum XAutoResponseSuppress {
    /// Suppresses auto responses from Exchange.
    /// See <https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxcmail/e489ffaf-19ed-4285-96d9-c31c42cab17f> for details.
    All,
}

impl Header for XAutoResponseSuppress {
    fn name() -> lettre::message::header::HeaderName {
        HeaderName::new_from_ascii_str("X-Auto-Response-Suppress")
    }

    fn parse(_: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err(anyhow!("Not supported").into())
    }

    fn display(&self) -> lettre::message::header::HeaderValue {
        HeaderValue::new(
            Self::name(),
            match self {
                Self::All => "All".to_owned(),
            },
        )
    }
}

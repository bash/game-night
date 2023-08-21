use anyhow::{Context as _, Result};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::TlsParameters;
use lettre::transport::smtp::AsyncSmtpTransportBuilder;
use lettre::{AsyncTransport, Message};
use rand::{distributions, thread_rng, Rng as _};
use rocket::async_trait;
use rocket::figment::Figment;
use rocket::warn;
use serde::Deserialize;
use tera::{Context, Tera};

pub(crate) const EMAIL_DISPLAY_TIMEZONE: chrono_tz::Tz = chrono_tz::Europe::Zurich;

type AsyncSmtpTransport = lettre::AsyncSmtpTransport<lettre::AsyncStd1Executor>;

#[async_trait]
pub(crate) trait EmailSender: Send + Sync {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()>;
}

pub(crate) trait EmailMessage: Send + Sync {
    fn subject(&self) -> String;

    fn template_name(&self) -> String;

    fn template_context(&self) -> Context {
        Context::new()
    }
}

pub(crate) struct EmailSenderImpl {
    sender: Mailbox,
    transport: AsyncSmtpTransport,
    tera: Tera,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()> {
        let message = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(email.subject())
            .multipart(render_email_body(&self.tera, email)?)
            .context("failed to create email message")?;
        self.transport
            .send(message)
            .await
            .context("failed to send email")
            .map(|_| ())
    }
}

fn render_email_body(tera: &Tera, email: &dyn EmailMessage) -> Result<MultiPart> {
    let template_name = email.template_name();
    let mut template_context = email.template_context();
    template_context.insert("greeting", get_random_greeting());
    template_context.insert("skin_tone", get_random_skin_tone_modifier());
    let html_template_name = format!("{}.html.tera", &template_name);
    let text_template_name = format!("{}.txt.tera", &template_name);

    Ok(MultiPart::alternative_plain_html(
        tera.render(&text_template_name, &template_context)
            .context("failed to render tera template")?,
        tera.render(&html_template_name, &template_context)
            .context("failed to render tera template")?,
    ))
}

impl EmailSenderImpl {
    pub(crate) async fn from_figment(figment: &Figment) -> Result<Self> {
        let config: EmailSenderConfig = figment
            .extract_inner("email")
            .context("failed to read email sender configuration")?;
        let sender = config.sender.clone();

        let transport: AsyncSmtpTransport = config.try_into()?;
        test_connection(&transport).await;

        Ok(Self {
            sender,
            transport,
            tera: create_tera()?,
        })
    }
}

async fn test_connection(transport: &AsyncSmtpTransport) {
    match transport.test_connection().await {
        Err(e) => warn!("unable to connect to configured SMTP transport:\n{}", e),
        Ok(successful) if !successful => {
            warn!("failed to connect to configured SMTP transport")
        }
        Ok(_) => {}
    }
}

fn create_tera() -> Result<Tera> {
    let mut tera = Tera::new("emails/**.tera").context("failed to initialize Tera")?;
    tera.build_inheritance_chains()
        .context("failed to build tera's inheritance chain")?;
    Ok(tera)
}

fn get_random_greeting() -> &'static str {
    const GREETINGS: &[&str] = &[
        "Hi",
        "Ciao",
        "Salü",
        "Hola",
        "Hellooo",
        "Hey there",
        "Greetings galore",
        "Aloha",
        "Howdy",
        "Hiyaa",
        "Yoohoo~",
        "Ahoy",
    ];
    thread_rng().sample(distributions::Slice::new(GREETINGS).unwrap())
}

fn get_random_skin_tone_modifier() -> &'static str {
    const SKIN_TONE_MODIFIERS: &[&str] = &[
        "\u{1F3FB}",
        "\u{1F3FC}",
        "\u{1F3FD}",
        "\u{1F3FE}",
        "\u{1F3FF}",
    ];
    thread_rng().sample(distributions::Slice::new(SKIN_TONE_MODIFIERS).unwrap())
}

#[derive(Debug, Deserialize)]
struct EmailSenderConfig {
    sender: Mailbox,
    smtp_server: String,
    smtp_port: u16,
    smtp_tls: EmailSenderTls,
    smtp_credentials: Option<EmailSenderCredentials>,
}

impl TryFrom<EmailSenderConfig> for AsyncSmtpTransport {
    type Error = anyhow::Error;

    fn try_from(config: EmailSenderConfig) -> Result<Self> {
        Ok(AsyncSmtpTransport::relay(&config.smtp_server)
            .context("Failed to create SMTP transport")?
            .port(config.smtp_port)
            .tls(config.smtp_tls.to_client_tls(config.smtp_server)?)
            .optional_credentials(config.smtp_credentials.map(Into::into))
            .build())
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EmailSenderTls {
    None,
    Required,
}

impl EmailSenderTls {
    fn to_client_tls(self, domain: String) -> Result<lettre::transport::smtp::client::Tls> {
        use lettre::transport::smtp::client::Tls;
        match self {
            EmailSenderTls::None => Ok(Tls::None),
            EmailSenderTls::Required => Ok(Tls::Required(
                TlsParameters::new(domain).context("Failed to create TLS parameters")?,
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
struct EmailSenderCredentials {
    username: String,
    password: String,
}

impl From<EmailSenderCredentials> for Credentials {
    fn from(EmailSenderCredentials { username, password }: EmailSenderCredentials) -> Self {
        Credentials::new(username, password)
    }
}

trait AsyncSmtpTransportBuilderExt {
    fn optional_credentials(self, credentials: Option<Credentials>) -> Self;
}

impl AsyncSmtpTransportBuilderExt for AsyncSmtpTransportBuilder {
    fn optional_credentials(self, credentials: Option<Credentials>) -> Self {
        match credentials {
            Some(credentials) => self.credentials(credentials),
            None => self,
        }
    }
}

use anyhow::{Context as _, Result};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::TlsParameters;
use lettre::transport::smtp::AsyncSmtpTransportBuilder;
use rocket::async_trait;
use rocket::figment::Figment;
use rocket::warn;
use serde::Deserialize;
use std::error::Error;
use tera::{Context, Tera};

type AsyncSmtpTransport = lettre::AsyncSmtpTransport<lettre::AsyncStd1Executor>;

#[async_trait]
pub(crate) trait EmailSender: Send + Sync {
    async fn send(&self, email: EmailMessage) -> Result<(), Box<dyn Error>>;
}

pub(crate) struct EmailMessage {
    pub(crate) recipient: Mailbox,
    pub(crate) subject: String,
    pub(crate) template_name: String,
    pub(crate) template_context: Context,
}

pub(crate) struct EmailSenderImpl {
    sender: Mailbox,
    transport: AsyncSmtpTransport,
    tera: Tera,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(&self, email: EmailMessage) -> Result<(), Box<dyn Error>> {
        todo!()
    }
}

impl EmailSenderImpl {
    pub(crate) async fn from_figment(figment: &Figment) -> Result<Self> {
        let config: EmailSenderConfig = figment
            .extract_inner("email")
            .context("Failed to read email sender configuration")?;
        let sender = config.sender.clone();

        let transport: AsyncSmtpTransport = config.try_into()?;
        match transport.test_connection().await {
            Err(e) => warn!("unable to connect to configured SMTP transport:\n{}", e),
            Ok(successful) if !successful => {
                warn!("failed to connect to configured SMTP transport")
            }
            Ok(_) => {}
        };

        let mut tera = Tera::new("emails/**.tera").context("Failed to initialize Tera")?;
        tera.build_inheritance_chains()
            .context("Failed to build tera's inheritance chain")?;
        Ok(Self {
            sender,
            transport,
            tera,
        })
    }
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

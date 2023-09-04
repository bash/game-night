use anyhow::{Context as _, Result};
use lettre::message::{Mailbox, MultiPart};
use lettre::Message;
use rand::{distributions, thread_rng, Rng as _};
use rocket::async_trait;
use rocket::figment::value::magic::RelativePathBuf;
use rocket::figment::Figment;
use rocket::tokio::fs::{create_dir_all, read_to_string, rename, OpenOptions};
use rocket::tokio::io::AsyncWriteExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use uuid::Uuid;

const DEFAULT_EMAIL_TEMPLATE_DIR: &str = "emails";
const DEFAULT_OUTBOX_DIR: &str = "outbox";
pub(crate) const EMAIL_DISPLAY_TIMEZONE: chrono_tz::Tz = chrono_tz::Europe::Zurich;

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
    outbox_dir: PathBuf,
    css: String,
    tera: Tera,
}

#[async_trait]
impl EmailSender for EmailSenderImpl {
    async fn send(&self, recipient: Mailbox, email: &dyn EmailMessage) -> Result<()> {
        let message = Message::builder()
            .from(self.sender.clone())
            .to(recipient)
            .subject(email.subject())
            .multipart(self.render_email_body(email)?)
            .context("failed to create email message")?;

        let message_id = Uuid::new_v4();

        create_dir_all(&self.outbox_dir).await?;

        // We don't want the outbox processor to attempt to send
        // files while we're writing them.
        // We use the leading _ to communicate that to the `outbox` program.
        let mut temporary_path = self.outbox_dir.clone();
        temporary_path.push(format!("_{}.eml", message_id));
        let mut file = OpenOptions::default()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .await?;
        file.write_all(&message.formatted()).await?;

        // Renames are atomic, so the file is available `outbox` all at once.
        let mut message_path = self.outbox_dir.clone();
        message_path.push(format!("{}.eml", message_id));
        rename(temporary_path, message_path).await?;

        Ok(())
    }
}

impl EmailSenderImpl {
    pub(crate) async fn from_figment(figment: &Figment) -> Result<Self> {
        let config: EmailSenderConfig = figment
            .extract_inner("email")
            .context("failed to read email sender configuration")?;
        let template_dir = config
            .template_dir
            .as_ref()
            .map(|t| t.relative())
            .unwrap_or_else(|| DEFAULT_EMAIL_TEMPLATE_DIR.into());
        let outbox_dir = config
            .outbox_dir
            .map(|d| d.relative())
            .unwrap_or_else(|| DEFAULT_OUTBOX_DIR.into());
        let mut css_file_path = template_dir.clone();
        css_file_path.push("email.css");

        Ok(Self {
            sender: config.sender,
            outbox_dir,
            tera: create_tera(&template_dir)?,
            css: read_to_string(css_file_path).await?,
        })
    }

    fn render_email_body(&self, email: &dyn EmailMessage) -> Result<MultiPart> {
        let template_name = email.template_name();
        let mut template_context = email.template_context();
        template_context.insert("greeting", get_random_greeting());
        template_context.insert("skin_tone", get_random_skin_tone_modifier());
        template_context.insert("css", &self.css);
        let html_template_name = format!("{}.html.tera", &template_name);
        let text_template_name = format!("{}.txt.tera", &template_name);

        Ok(MultiPart::alternative_plain_html(
            self.tera
                .render(&text_template_name, &template_context)
                .context("failed to render tera template")?,
            self.tera
                .render(&html_template_name, &template_context)
                .context("failed to render tera template")?,
        ))
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
    outbox_dir: Option<RelativePathBuf>,
    template_dir: Option<RelativePathBuf>,
}

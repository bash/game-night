use super::EmailMessage;
use crate::decorations::{Closings, Greetings, Hearts, SkinToneModifiers};
use crate::infra::configure_tera;
use anyhow::{Context as _, Result};
use lettre::message::MultiPart;
use rand::{rng, Rng as _};
use rocket::tokio::fs::read_to_string;
use rocket_dyn_templates::tera::Tera;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct EmailRenderer {
    css: String,
    tera: Tera,
}

#[derive(Debug, Clone)]
pub(crate) struct EmailBody {
    pub(crate) plain: String,
    pub(crate) html: String,
}

impl EmailRenderer {
    pub(crate) async fn from_template_dir(template_dir: &Path) -> Result<Self> {
        let mut css_file_path = template_dir.to_owned();
        css_file_path.push("email.css");
        Ok(Self {
            tera: create_tera(template_dir)?,
            css: read_to_string(css_file_path)
                .await
                .context("email.css is missing")?,
        })
    }

    pub(crate) fn render(&self, email: &dyn EmailMessage) -> Result<EmailBody> {
        let template_name = email.template_name();
        let mut template_context = email.template_context()?;
        let mut rng = rng();
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

fn create_tera(template_dir: &Path) -> Result<Tera> {
    let templates = template_dir.join("**.tera");
    let templates = templates
        .to_str()
        .context("template dir is not valid utf-8")?;
    let mut tera = Tera::new(templates).context("failed to initialize Tera")?;
    tera.build_inheritance_chains()
        .context("failed to build tera's inheritance chain")?;
    configure_tera(&mut tera)?;
    Ok(tera)
}

impl From<EmailBody> for MultiPart {
    fn from(value: EmailBody) -> Self {
        MultiPart::alternative_plain_html(value.plain, value.html)
    }
}

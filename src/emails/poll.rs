use crate::email::EmailMessage;
use tera::Context;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub(crate) struct PollEmail {
    pub(crate) name: String,
    pub(crate) poll_closes_at: OffsetDateTime,
    pub(crate) poll_url: String,
}

impl EmailMessage for PollEmail {
    fn subject(&self) -> String {
        "Get Ready For Another Game Night".to_owned()
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }

    fn template_context(&self) -> Context {
        let mut context = Context::new();
        context.insert("name", &self.name);
        context.insert("poll_url", &self.poll_url);
        context
    }
}

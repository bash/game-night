use crate::email::{EmailMessage, EMAIL_DISPLAY_TIMEZONE};
use chrono::{DateTime, Local};
use tera::Context;

#[derive(Debug, Clone)]
pub(crate) struct PollEmail {
    pub(crate) greeting: String,
    pub(crate) name: String,
    pub(crate) poll_closes_at: DateTime<Local>,
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
        let poll_closes_at = self.poll_closes_at.with_timezone(&EMAIL_DISPLAY_TIMEZONE);
        let mut context = Context::new();
        context.insert("greeting", &self.greeting);
        context.insert("name", &self.name);
        context.insert(
            "poll_close_date",
            &poll_closes_at.format("%d. %B %Y").to_string(),
        );
        context.insert(
            "poll_close_time",
            &poll_closes_at.format("%I:%M %p").to_string(),
        );
        context.insert("poll_url", &self.poll_url);
        context
    }
}

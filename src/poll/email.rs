use crate::email::EmailMessage;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail {
    pub(super) name: String,
    #[serde(with = "time::serde::iso8601")]
    pub(super) poll_closes_at: OffsetDateTime,
    pub(super) poll_url: String,
}

impl EmailMessage for PollEmail {
    fn subject(&self) -> String {
        "Are You Ready for a Game Night?".to_owned()
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }
}

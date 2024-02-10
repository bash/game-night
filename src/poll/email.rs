use crate::email::EmailMessage;
use rocket::http::uri::Absolute;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail<'a> {
    pub(super) name: String,
    #[serde(with = "time::serde::iso8601")]
    pub(super) poll_closes_at: OffsetDateTime,
    pub(super) poll_url: Absolute<'a>,
}

impl EmailMessage for PollEmail<'_> {
    fn subject(&self) -> String {
        "Are You Ready for a Game Night?".to_owned()
    }

    fn template_name(&self) -> String {
        "poll".to_owned()
    }
}

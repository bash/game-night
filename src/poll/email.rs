use crate::email::EmailMessage;
use serde::{Serialize, Serializer};
use tera::Context;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, OffsetDateTimeExt};

#[derive(Debug, Clone, Serialize)]
pub(super) struct PollEmail {
    pub(super) name: String,
    #[serde(serialize_with = "serialize_as_cet")]
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

    fn template_context(&self) -> Context {
        Context::from_serialize(self).unwrap()
    }
}

fn serialize_as_cet<S: Serializer>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
    let in_cet = dt.to_timezone(timezones::db::CET);
    let primitive = PrimitiveDateTime::new(in_cet.date(), in_cet.time());
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let formatted = primitive.format(&format).unwrap();
    serializer.serialize_str(&formatted)
}

use serde::Serializer;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use time_tz::{timezones, OffsetDateTimeExt as _};

pub(crate) fn serialize_as_cet<S: Serializer>(
    dt: &OffsetDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let in_cet = dt.to_timezone(timezones::db::CET);
    let primitive = PrimitiveDateTime::new(in_cet.date(), in_cet.time());
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    let formatted = primitive.format(&format).unwrap();
    serializer.serialize_str(&formatted)
}

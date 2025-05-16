use super::EmailSubscription;
use crate::iso_8601::Iso8601;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::{Database, Decode, Encode, Type};
use std::fmt;
use std::str::FromStr;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::Date;

impl fmt::Display for EmailSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EmailSubscription::*;
        match self {
            Subscribed => f.write_str("subscribed"),
            PermanentlyUnsubscribed => f.write_str("unsubscribed"),
            TemporarilyUnsubscribed { until: date } => write!(f, "{date}"),
        }
    }
}

impl FromStr for EmailSubscription {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmailSubscription::*;
        match s {
            "subscribed" => Ok(Subscribed),
            "unsubscribed" => Ok(PermanentlyUnsubscribed),
            other => Ok(TemporarilyUnsubscribed {
                until: Iso8601::<Date>::from_str(other)?,
            }),
        }
    }
}

impl<DB: Database> Type<DB> for EmailSubscription
where
    for<'a> &'a str: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <&str as Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <&str as Type<DB>>::compatible(ty)
    }
}

impl<'q, DB: Database> Encode<'q, DB> for EmailSubscription
where
    &'q str: Encode<'q, DB>,
    Date: Encode<'q, DB>,
{
    fn encode_by_ref(&self, buf: &mut DB::ArgumentBuffer<'q>) -> Result<IsNull, BoxDynError> {
        use EmailSubscription::*;
        match self {
            Subscribed => "subscribed".encode_by_ref(buf),
            PermanentlyUnsubscribed => "unsubscribed".encode_by_ref(buf),
            TemporarilyUnsubscribed { until: date } => date.encode_by_ref(buf),
        }
    }
}

impl<'r, DB: Database> Decode<'r, DB> for EmailSubscription
where
    &'r str: Decode<'r, DB>,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        const FORMAT: &[FormatItem<'static>] = format_description!("[year]-[month]-[day]");
        use EmailSubscription::*;
        match <&str as Decode<'r, DB>>::decode(value)? {
            "subscribed" => Ok(Subscribed),
            "unsubscribed" => Ok(PermanentlyUnsubscribed),
            other => Ok(TemporarilyUnsubscribed {
                until: Date::parse(other, FORMAT)?.into(),
            }),
        }
    }
}

use super::EmailSubscription;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::{Database, Decode, Encode, Type};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::Date;

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

use rocket::FromForm;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use time::format_description::BorrowedFormatItem as FormatItem;
use time::macros::format_description;
use time::{Date, OffsetDateTime, Time};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, sqlx::Type, FromForm)]
#[sqlx(transparent)]
#[form(transparent)]
pub(crate) struct Iso8601<T>(pub(crate) T);

impl<T> From<T> for Iso8601<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Iso8601<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Iso8601<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

macro_rules! impl_serde {
    ($T:ident with $mod:path) => {
        impl Serialize for Iso8601<$T> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use $mod as module;
                module::serialize(self, serializer)
            }
        }

        impl<'de> Deserialize<'de> for Iso8601<$T> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use $mod as module;
                module::deserialize(deserializer).map(Self)
            }
        }
    };
}

macro_rules! impl_display {
    ($T:ident with $format:expr) => {
        impl fmt::Display for Iso8601<$T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let formatted = self.0.format($format).map_err(|_| fmt::Error)?;
                f.write_str(&formatted)
            }
        }
    };
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");
time::serde::format_description!(iso8601_time, Time, "[hour]:[minute]");

impl_serde!(OffsetDateTime with time::serde::iso8601);
impl_serde!(Date with iso8601_date);
impl_serde!(Time with iso8601_time);

const DATE_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
impl_display!(Date with DATE_FORMAT);
const TIME_FORMAT: &[FormatItem<'_>] = format_description!("[hour]:[minute]");
impl_display!(Time with TIME_FORMAT);

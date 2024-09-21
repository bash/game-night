use rocket::FromForm;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use time::{Date, OffsetDateTime, Time};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, sqlx::Type, FromForm)]
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

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");
time::serde::format_description!(iso8601_time, Time, "[hour]:[minute]");

impl_serde!(OffsetDateTime with time::serde::iso8601);
impl_serde!(Date with iso8601_date);
impl_serde!(Time with iso8601_time);

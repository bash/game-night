use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use rocket::FromForm;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use time::format_description::well_known::Iso8601 as Iso8601Format;
use time::{Date, OffsetDateTime, Time};

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, sqlx::Type, FromForm, AsExpression,
)]
#[sqlx(transparent)]
#[form(transparent)]
#[diesel(sql_type = Text)]
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

        impl std::str::FromStr for Iso8601<$T> {
            type Err = time::error::Parse;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                $T::parse(s, $format).map(Iso8601)
            }
        }

        $crate::impl_to_from_sql! { Iso8601<$T> }

        impl<ST, DB> diesel::deserialize::Queryable<ST, DB> for Iso8601<$T>
        where
            DB: diesel::backend::Backend,
            String: diesel::deserialize::Queryable<ST, DB>,
        {
            type Row = <String as diesel::deserialize::Queryable<ST, DB>>::Row;
            fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
                use std::str::FromStr as _;
                let raw_value = String::build(row)?;
                Ok(Iso8601::from_str(&raw_value)?)
            }
        }
    };
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");
time::serde::format_description!(iso8601_time, Time, "[hour]:[minute]");

impl_serde!(OffsetDateTime with time::serde::iso8601);
impl_serde!(Date with iso8601_date);
impl_serde!(Time with iso8601_time);

impl_display!(OffsetDateTime with &Iso8601Format::DATE_TIME_OFFSET);
impl_display!(Date with &Iso8601Format::DATE);
impl_display!(Time with &Iso8601Format::TIME);

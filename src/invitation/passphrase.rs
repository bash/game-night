use super::Invitation;
use anyhow::Result;
use rocket::form::FromFormField;
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::{Query, UriDisplay};
use serde::Serialize;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Database, Decode, Encode, Sqlite};
use std::fmt;

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub(crate) struct Passphrase(pub(crate) Vec<String>);

impl fmt::Display for Passphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join(" "))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Passphrase
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<Passphrase, Box<dyn std::error::Error + 'static + Send + Sync>> {
        Ok(Self(
            <&str as Decode<DB>>::decode(value)?
                .split(' ')
                .map(ToOwned::to_owned)
                .collect(),
        ))
    }
}

impl<'q> Encode<'q, Sqlite> for Passphrase
where
    &'q str: Encode<'q, Sqlite>,
{
    fn encode_by_ref(&self, buf: &mut <Sqlite as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
        buf.push(SqliteArgumentValue::Text(self.0.join(" ").into()));
        IsNull::No
    }
}

impl sqlx::Type<Sqlite> for Passphrase {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
}

#[derive(Debug)]
pub(crate) struct Passphrases(pub(crate) Vec<Passphrase>);

impl Passphrases {
    pub(crate) fn from_invitations<'a>(iter: impl Iterator<Item = &'a Invitation>) -> Self {
        Self(iter.map(|i| i.passphrase.clone()).collect())
    }
}

impl<'v> FromFormField<'v> for Passphrases {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Passphrases(
            field
                .value
                .split(',')
                .map(|p| Passphrase(p.split('-').map(ToOwned::to_owned).collect()))
                .collect(),
        ))
    }
}

impl_from_uri_param_identity!([Query] Passphrases);

impl UriDisplay<Query> for Passphrases {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> fmt::Result {
        let value = self
            .0
            .iter()
            .map(|p| p.0.join("-"))
            .collect::<Vec<_>>()
            .join(",");
        f.write_value(value)
    }
}

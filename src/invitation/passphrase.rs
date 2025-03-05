use super::TAUS_WORDLIST;
use anyhow::Result;
use rand::distr::{Distribution, StandardUniform};
use rand::seq::IndexedRandom as _;
use rand::Rng;
use rocket::form::FromFormField;
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::{Query, UriDisplay};
use serde::Serialize;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Database, Decode, Encode, Sqlite};
use std::fmt;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(transparent)]
pub(crate) struct Passphrase(pub(crate) Vec<String>);

impl Distribution<Passphrase> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Passphrase {
        let words: Vec<_> = TAUS_WORDLIST
            .choose_multiple(rng, 4)
            .map(|s| s.to_string())
            .collect();
        Passphrase(words)
    }
}

impl fmt::Display for Passphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join(" "))
    }
}

impl<'v> FromFormField<'v> for Passphrase {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Passphrase::from_form_field(field.value))
    }
}

impl_from_uri_param_identity!([Query] Passphrase);

impl UriDisplay<Query> for Passphrase {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> fmt::Result {
        f.write_value(self.to_form_field())
    }
}

impl Passphrase {
    fn from_form_field(value: &str) -> Self {
        Self::from_form_fields(value.split('-'))
    }

    pub(crate) fn from_form_fields<'a>(values: impl Iterator<Item = &'a str>) -> Self {
        Self(values.map(|w| w.to_lowercase().trim().to_owned()).collect())
    }

    fn to_form_field(&self) -> String {
        self.0.join("-")
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Passphrase
where
    &'r str: Decode<'r, DB>,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Passphrase, BoxDynError> {
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
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, BoxDynError> {
        buf.push(SqliteArgumentValue::Text(self.0.join(" ").into()));
        Ok(IsNull::No)
    }
}

impl sqlx::Type<Sqlite> for Passphrase {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
}

use rocket::form::{FromFormField, ValueField};
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::{FromUriParam, Query, UriDisplay};
use rocket::http::uri::{Origin, Reference};
use rocket::{async_trait, form, uri};
use std::fmt;

pub(crate) trait RedirectUriExt {
    fn or_root(self) -> RedirectUri;
}

impl RedirectUriExt for Option<RedirectUri> {
    fn or_root(self) -> RedirectUri {
        self.unwrap_or(RedirectUri(uri!("/")))
    }
}

impl From<RedirectUri> for Reference<'static> {
    fn from(value: RedirectUri) -> Self {
        value.0.into()
    }
}

impl From<RedirectUri> for Origin<'static> {
    fn from(value: RedirectUri) -> Self {
        value.0
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RedirectUri(pub(crate) Origin<'static>);

impl_from_uri_param_identity!([Query] RedirectUri);

impl UriDisplay<Query> for RedirectUri {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> fmt::Result {
        f.write_value(self.0.to_string())
    }
}

impl<'a> FromUriParam<Query, &'a Origin<'a>> for RedirectUri {
    type Target = String;

    fn from_uri_param(param: &'a Origin<'a>) -> Self::Target {
        param.to_string()
    }
}

impl<'a> FromUriParam<Query, Origin<'a>> for RedirectUri {
    type Target = String;

    fn from_uri_param(param: Origin<'a>) -> Self::Target {
        param.to_string()
    }
}

#[async_trait]
impl<'r> FromFormField<'r> for RedirectUri {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        Origin::parse_owned(field.value.to_string())
            .map(RedirectUri)
            .map_err(|_| form::Errors::from(form::Error::validation("Invalid redirect URI")))
    }
}

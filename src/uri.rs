use anyhow::{Error, Result};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Phase, Request, Rocket};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct UrlPrefix<'a>(pub(crate) Absolute<'a>);

impl<'a> UrlPrefix<'a> {
    pub(crate) fn to_static(&self) -> UrlPrefix<'static> {
        UrlPrefix(Absolute::parse_owned(self.0.to_string()).unwrap())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for UrlPrefix<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.rocket().url_prefix() {
            Ok(value) => Outcome::Success(value),
            Err(e) => Outcome::Error((Status::InternalServerError, e)),
        }
    }
}

pub(crate) trait HasUrlPrefix {
    fn url_prefix(&self) -> Result<UrlPrefix<'_>>;
}

impl<P: Phase> HasUrlPrefix for Rocket<P> {
    fn url_prefix(&self) -> Result<UrlPrefix<'_>> {
        self.figment()
            .extract_inner("url_prefix")
            .map(UrlPrefix)
            .map_err(Into::into)
    }
}

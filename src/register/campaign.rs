use anyhow::{Error, Result};
use rocket::figment::{self, Figment};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub(super) struct ProvidedCampaign<'a>(Option<Campaign<'a>>);

impl<'a> ProvidedCampaign<'a> {
    pub(super) fn into_inner(self) -> Option<Campaign<'a>> {
        self.0
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Campaign<'a> {
    pub(super) name: &'a str,
    pub(super) message: String,
}

#[async_trait]
impl<'r> FromRequest<'r> for ProvidedCampaign<'r> {
    type Error = Option<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let campaign: &str = match campaing_name_from_query(request) {
            Some(c) => c,
            None => return Outcome::Success(ProvidedCampaign(None)),
        };
        match campaign_from_figment(request.rocket().figment(), campaign) {
            Ok(Some(c)) => Outcome::Success(ProvidedCampaign(Some(c))),
            Ok(None) => Outcome::Error((Status::BadRequest, None)),
            Err(e) => {
                dbg!(&e);
                Outcome::Error((Status::ServiceUnavailable, Some(e)))
            }
        }
    }
}

fn campaing_name_from_query<'r>(request: &'r Request<'_>) -> Option<&'r str> {
    request
        .query_value("campaign")
        .map(|r| r.expect("Infallible conversion"))
}

fn campaign_from_figment<'a>(figment: &'a Figment, name: &'a str) -> Result<Option<Campaign<'a>>> {
    match figment.focus("campaign").extract_inner(name) {
        Ok(ConfiguredCampaign { message }) => Ok(Some(Campaign { name, message })),
        Err(figment::Error {
            kind: figment::error::Kind::MissingField(..),
            ..
        }) => return Ok(None),
        Err(e) => return Err(e.into()),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ConfiguredCampaign {
    message: String,
}

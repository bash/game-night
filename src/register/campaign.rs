use anyhow::{anyhow, Error, Result};
use rocket::figment::{self, Figment};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub(super) struct ProvidedCampaign<'a>(Option<Campaign<'a>>);

impl<'a> ProvidedCampaign<'a> {
    pub(super) fn into_inner(self) -> Option<Campaign<'a>> {
        self.0
    }
}

#[derive(Debug, Clone)]
pub(super) struct Campaign<'a> {
    pub(super) name: &'a str,
    pub(super) message: String,
}

#[async_trait]
impl<'r> FromRequest<'r> for ProvidedCampaign<'r> {
    type Error = Option<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let campaign: &str = match campaing_name_from_query(request) {
            Ok(Some(c)) => c,
            Ok(None) => return Outcome::Success(ProvidedCampaign(None)),
            Err(e) => return Outcome::Error((Status::BadRequest, Some(e))),
        };
        match campaign_from_figment(request.rocket().figment(), campaign) {
            Ok(Some(c)) => Outcome::Success(ProvidedCampaign(Some(c))),
            Ok(None) => Outcome::Error((Status::BadRequest, None)),
            Err(e) => Outcome::Error((Status::ServiceUnavailable, Some(e))),
        }
    }
}

fn campaing_name_from_query<'r>(request: &'r Request<'_>) -> Result<Option<&'r str>> {
    request
        .query_value("campaign")
        .transpose()
        .map_err(|e| anyhow!("failed to read form value: {e}"))
}

fn campaign_from_figment<'a>(figment: &'a Figment, name: &'a str) -> Result<Option<Campaign<'a>>> {
    match figment.focus("campaign").extract_inner(name) {
        Ok(ConfiguredCampaign { message }) => Ok(Some(Campaign { name, message })),
        Err(figment::Error {
            kind: figment::error::Kind::MissingField(..),
            ..
        }) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ConfiguredCampaign {
    message: String,
}

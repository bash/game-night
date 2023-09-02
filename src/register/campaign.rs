use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(super) struct Campaign<'a>(Option<&'a str>);

impl<'a> Campaign<'a> {
    pub(super) fn into_inner(self) -> Option<&'a str> {
        self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Campaign<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let campaign: Option<&str> = request.query_value("campaign").map(|r| r.unwrap());
        let allowed_campaigns: Vec<String> = request
            .rocket()
            .figment()
            .extract_inner("allowed_campaigns")
            .unwrap_or_default();

        if campaign
            .as_ref()
            .map(|c| allowed_campaigns.iter().any(|allowed| c == allowed))
            .unwrap_or(true)
        {
            Outcome::Success(Campaign(campaign))
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

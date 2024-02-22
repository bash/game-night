use super::Poll;
use crate::database::Repository;
use anyhow::Error;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use serde::Serialize;
use std::ops;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(crate) struct Open<T>(T);

impl<T> Open<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for Open<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Open<Poll> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut repository: Box<dyn Repository> =
            try_outcome!(FromRequest::from_request(request).await);
        match repository.get_open_poll().await {
            Ok(Some(poll)) => Outcome::Success(Open(poll)),
            Ok(None) => Outcome::Forward(Status::NotFound),
            Err(error) => Outcome::Error((Status::InternalServerError, error)),
        }
    }
}

use super::User;
use crate::{database::Repository, iso_8601::Iso8601};
use anyhow::{Error, Result};
use rocket::{
    async_trait,
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome},
    Request,
};
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum EmailSubscription {
    #[default]
    Subscribed,
    TemporarilyUnsubscribed {
        until: Iso8601<Date>,
    },
    PermanentlyUnsubscribed,
}

impl EmailSubscription {
    pub(crate) fn is_subscribed(&self, today: Date) -> bool {
        match self {
            EmailSubscription::Subscribed => true,
            EmailSubscription::TemporarilyUnsubscribed { until } => today > **until,
            EmailSubscription::PermanentlyUnsubscribed => false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SubscribedUsers(pub(crate) Vec<User>);

#[async_trait]
impl<'r> FromRequest<'r> for SubscribedUsers {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut database: Box<dyn Repository> = try_outcome!(request.guard().await);
        let users = try_outcome!(get_subscribed_users(&mut *database)
            .await
            .or_error(Status::InternalServerError));
        Outcome::Success(SubscribedUsers(users.collect()))
    }
}

async fn get_subscribed_users(
    repository: &mut dyn Repository,
) -> Result<impl Iterator<Item = User>> {
    let today = OffsetDateTime::now_utc().date();
    Ok(repository
        .get_users()
        .await?
        .into_iter()
        .filter(move |u| u.email_subscription.is_subscribed(today)))
}

use super::{User, UsersQuery};
use crate::event::StatefulEvent;
use crate::iso_8601::Iso8601;
use anyhow::{Error, Result};
use rocket::async_trait;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
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

#[derive(Debug)]
pub(crate) struct SubscribedUsers {
    users: UsersQuery,
}

impl SubscribedUsers {
    pub(crate) async fn for_event(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let today = OffsetDateTime::now_utc().date();
        let is_subscribed = |u: &User| u.email_subscription.is_subscribed(today);
        let invited = self.users.invited(event).await?;
        Ok(invited.into_iter().filter(is_subscribed).collect())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for SubscribedUsers {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let users = try_outcome!(request.guard().await);
        Outcome::Success(SubscribedUsers { users })
    }
}

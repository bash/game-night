use super::User;
use crate::database::{Materialized, Repository};
use crate::event::{Event, EventLifecycle};
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
    repository: Box<dyn Repository>,
}

impl SubscribedUsers {
    pub(crate) async fn for_event<L: EventLifecycle>(
        &mut self,
        event: &Event<Materialized, L>,
    ) -> Result<Vec<User>> {
        let today = OffsetDateTime::now_utc().date();
        let is_subscribed = |u: &User| u.email_subscription.is_subscribed(today);
        if let Some(group) = &event.restrict_to {
            Ok(group
                .members
                .iter()
                .filter(|u| is_subscribed(u))
                .cloned()
                .collect())
        } else {
            let users = self.repository.get_users().await?;
            Ok(users.into_iter().filter(is_subscribed).collect())
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for SubscribedUsers {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let repository = try_outcome!(request.guard().await);
        Outcome::Success(SubscribedUsers { repository })
    }
}

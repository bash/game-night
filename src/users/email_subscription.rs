use super::{User, UsersQuery};
use crate::auto_resolve;
use crate::event::StatefulEvent;
use crate::iso_8601::Iso8601;
use anyhow::Result;
use time::{Date, OffsetDateTime};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
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

auto_resolve! {
    #[derive(Debug)]
    pub(crate) struct SubscribedUsers {
        users: UsersQuery,
    }
}

impl SubscribedUsers {
    pub(crate) async fn for_event(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let today = OffsetDateTime::now_utc().date();
        let is_subscribed = |u: &User| u.email_subscription.is_subscribed(today);
        let invited = self.users.invited(event).await?;
        Ok(invited.into_iter().filter(is_subscribed).collect())
    }
}

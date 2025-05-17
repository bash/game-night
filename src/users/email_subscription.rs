use super::models::User;
use super::UserQueries;
use crate::event::StatefulEvent;
use crate::iso_8601::Iso8601;
use crate::{auto_resolve, impl_to_from_sql};
use anyhow::Result;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use std::fmt;
use std::str::FromStr;
use time::{Date, OffsetDateTime};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, FromSqlRow, AsExpression)]
#[diesel(sql_type = Text)]
pub(crate) enum EmailSubscription {
    #[default]
    Subscribed,
    TemporarilyUnsubscribed {
        until: Iso8601<Date>,
    },
    PermanentlyUnsubscribed,
}

impl_to_from_sql! { EmailSubscription }

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
    pub(crate) struct SubscribedUsers {
        users: UserQueries,
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

impl fmt::Display for EmailSubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EmailSubscription::*;
        match self {
            Subscribed => f.write_str("subscribed"),
            PermanentlyUnsubscribed => f.write_str("unsubscribed"),
            TemporarilyUnsubscribed { until: date } => write!(f, "{date}"),
        }
    }
}

impl FromStr for EmailSubscription {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmailSubscription::*;
        match s {
            "subscribed" => Ok(Subscribed),
            "unsubscribed" => Ok(PermanentlyUnsubscribed),
            other => Ok(TemporarilyUnsubscribed {
                until: Iso8601::<Date>::from_str(other)?,
            }),
        }
    }
}

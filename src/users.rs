use crate::auth::{AuthorizedTo, ManageUsers};
use crate::database::Repository;
use crate::template::PageBuilder;
use anyhow::{Error, Result};
use lettre::message::Mailbox;
use rocket::form;
use rocket::response::Debug;
use rocket::{async_trait, get, routes, Route};
use rocket_db_pools::sqlx;
use rocket_dyn_templates::{context, Template};
use serde::{Deserialize, Serialize};
use time::Date;

mod email_subscription;
mod last_activity;

pub(crate) fn routes() -> Vec<Route> {
    routes![list_users]
}

#[get("/users")]
async fn list_users(
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
    _guard: AuthorizedTo<ManageUsers>,
) -> Result<Template, Debug<Error>> {
    Ok(page.render("users", context! { users: repository.get_users().await? }))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, sqlx::Type, Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[async_trait]
impl<'v> form::FromFormField<'v> for UserId {
    fn from_value(field: form::ValueField<'v>) -> form::Result<'v, Self> {
        i64::from_value(field).map(UserId)
    }

    async fn from_data(field: form::DataField<'v, '_>) -> form::Result<'v, Self> {
        i64::from_data(field).await.map(UserId)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct User<Id = UserId> {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) email_subscription: EmailSubscription,
    pub(crate) invited_by: Option<UserId>,
    pub(crate) campaign: Option<String>,
    pub(crate) can_update_name: bool,
    pub(crate) can_answer_strongly: bool,
}

#[derive(Debug)]
pub(crate) struct UserPatch {
    pub(crate) name: Option<String>,
    pub(crate) email_subscription: Option<EmailSubscription>,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, sqlx::Type, Serialize)]
#[sqlx(rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
    #[default]
    Guest,
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum EmailSubscription {
    #[default]
    Subscribed,
    TemporarilyUnsubscribed {
        #[serde(with = "iso8601_date")]
        until: Date,
    },
    PermanentlyUnsubscribed,
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

impl EmailSubscription {
    pub(crate) fn is_subscribed(&self, today: Date) -> bool {
        match self {
            EmailSubscription::Subscribed => true,
            EmailSubscription::TemporarilyUnsubscribed { until } => today > *until,
            EmailSubscription::PermanentlyUnsubscribed => false,
        }
    }
}

impl<Id> User<Id> {
    pub(crate) fn mailbox(&self) -> Result<Mailbox> {
        Ok(Mailbox::new(
            Some(self.name.clone()),
            self.email_address.parse()?,
        ))
    }

    pub(crate) fn can_invite(&self) -> bool {
        self.role == Role::Admin
    }

    pub(crate) fn can_manage_poll(&self) -> bool {
        self.role == Role::Admin
    }

    pub(crate) fn can_manage_users(&self) -> bool {
        self.role == Role::Admin
    }

    pub(crate) fn can_answer_strongly(&self) -> bool {
        self.can_answer_strongly || self.role == Role::Admin
    }

    pub(crate) fn can_update_name(&self) -> bool {
        self.can_update_name
    }
}

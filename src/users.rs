use crate::auth::{is_invited, AuthorizedTo, ManageUsers};
use crate::database::Repository;
use crate::iso_8601::Iso8601;
use crate::template::PageBuilder;
use anyhow::{Error, Result};
use lettre::message::Mailbox;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::response::Debug;
use rocket::{async_trait, get, routes, Request, Route};
use rocket_db_pools::sqlx;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

mod email_subscription;
pub(crate) use email_subscription::*;
mod email_subscription_encoding;
mod last_activity;
pub(crate) use last_activity::*;
mod symbol;
use crate::event::StatefulEvent;
pub(crate) use symbol::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![list_users]
}

#[get("/users")]
async fn list_users(
    page: PageBuilder<'_>,
    mut users: UsersQuery,
    _guard: AuthorizedTo<ManageUsers>,
) -> Result<Template, Debug<Error>> {
    Ok(page.render("users", context! { users: users.active().await? }))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, sqlx::Type, Serialize, rocket::FromForm)]
#[sqlx(transparent)]
#[serde(transparent)]
#[form(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct User<Id = UserId> {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) symbol: AstronomicalSymbol,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) email_subscription: EmailSubscription,
    pub(crate) invited_by: Option<UserId>,
    pub(crate) campaign: Option<String>,
    pub(crate) can_update_name: bool,
    pub(crate) can_answer_strongly: bool,
    pub(crate) can_update_symbol: bool,
    pub(crate) last_active_at: Iso8601<OffsetDateTime>,
}

#[derive(Debug)]
pub(crate) struct UserPatch {
    pub(crate) name: Option<String>,
    pub(crate) symbol: Option<AstronomicalSymbol>,
    pub(crate) email_subscription: Option<EmailSubscription>,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, sqlx::Type, Serialize)]
#[sqlx(rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
    #[default]
    Guest,
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

    pub(crate) fn can_update_symbol(&self) -> bool {
        self.can_update_symbol
    }
}

pub(crate) const INACTIVITY_THRESHOLD: Duration = Duration::days(9 * 30);

#[derive(Debug)]
pub(crate) struct UsersQuery(Box<dyn Repository>);

impl UsersQuery {
    pub(crate) async fn all(&mut self) -> Result<Vec<User>> {
        self.0.get_users().await
    }

    pub(crate) async fn active(&mut self) -> Result<Vec<User>> {
        self.0.get_active_users(INACTIVITY_THRESHOLD).await
    }

    pub(crate) async fn invited(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let is_invited = |u: &User| is_invited(u, event);
        let users = self.all().await?;
        Ok(users.into_iter().filter(is_invited).collect())
    }

    pub(crate) async fn active_and_invited(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let is_invited = |u: &User| is_invited(u, event);
        let active = self.active().await?;
        Ok(active.into_iter().filter(is_invited).collect())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for UsersQuery {
    type Error = <Box<dyn Repository> as FromRequest<'r>>::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let repository = try_outcome!(request.guard().await);
        Outcome::Success(Self(repository))
    }
}

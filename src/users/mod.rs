use crate::auth::is_invited;
use crate::database::Repository;
use crate::event::StatefulEvent;
use crate::iso_8601::Iso8601;
use crate::{auto_resolve, impl_to_from_sql};
use anyhow::Result;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use diesel_derive_newtype::DieselNewType;
use lettre::message::Mailbox;
use rocket::{routes, Route};
use rocket_db_pools::sqlx;
use std::fmt;
use strum_lite::strum;
use time::{Duration, OffsetDateTime};

mod email_subscription;
pub(crate) use email_subscription::*;
mod email_subscription_encoding;
mod last_activity;
pub(crate) use last_activity::*;
mod symbol;
pub(crate) use symbol::*;
mod list;
pub(crate) use list::*;
mod name;
pub(crate) use name::*;
mod admin_user;
pub(crate) use admin_user::*;
mod commands;
pub(crate) use commands::*;
pub(crate) mod models;

pub(crate) fn routes() -> Vec<Route> {
    routes![list::list_users]
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, sqlx::Type, rocket::FromForm, DieselNewType)]
#[sqlx(transparent)]
#[form(transparent)]
pub(crate) struct UserId(pub(crate) i64);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

strum! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, sqlx::Type, FromSqlRow, AsExpression)]
    #[diesel(sql_type = Text)]
    #[sqlx(rename_all = "lowercase")]
    pub(crate) enum Role {
        Admin = "admin",
        #[default]
        Guest = "guest",
    }
}

impl_to_from_sql! { Role }

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

auto_resolve! {
    #[derive(Debug)]
    pub(crate) struct UsersQuery {
        repository: Box<dyn Repository>,
    }
}

impl UsersQuery {
    pub(crate) async fn all(&mut self) -> Result<Vec<User>> {
        self.repository.get_users().await
    }

    pub(crate) async fn active(&mut self) -> Result<Vec<User>> {
        self.repository.get_active_users(INACTIVITY_THRESHOLD).await
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

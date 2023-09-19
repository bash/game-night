use crate::authorization::{AuthorizedTo, ManageUsers};
use crate::database::Repository;
use crate::template::{PageBuilder, PageType};
use anyhow::{Error, Result};
use lettre::message::Mailbox;
use rocket::response::Debug;
use rocket::{get, routes, Route};
use rocket_db_pools::sqlx;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

pub(crate) fn routes() -> Vec<Route> {
    routes![list_users]
}

#[get("/users")]
async fn list_users(
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
    _guard: AuthorizedTo<ManageUsers>,
) -> Result<Template, Debug<Error>> {
    Ok(page
        .type_(PageType::Register)
        .render("users", context! { users: repository.get_users().await? }))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, sqlx::Type, Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct UserId(pub(crate) i64);

pub(crate) trait AsUserId {
    fn as_user_id(&self) -> UserId;
}

impl AsUserId for UserId {
    fn as_user_id(&self) -> UserId {
        *self
    }
}

impl AsUserId for User {
    fn as_user_id(&self) -> UserId {
        self.id
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct User<Id = UserId> {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) invited_by: Option<UserId>,
    pub(crate) campaign: Option<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, sqlx::Type, Serialize)]
#[sqlx(rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
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
}

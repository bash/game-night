use anyhow::Result;
use lettre::message::Mailbox;
use rocket_db_pools::sqlx;
use serde::Serialize;

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[derive(Debug, sqlx::FromRow, Serialize)]
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
}

use rocket_db_pools::sqlx;
use serde::Serialize;

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct User<Id = UserId> {
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) invited_by: Option<UserId>,
}

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize)]
#[sqlx(rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
    Guest,
}

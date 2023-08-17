use rocket_db_pools::sqlx;

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct User<Id = UserId> {
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) invited_by: Option<UserId>,
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
    Guest,
}

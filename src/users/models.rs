use super::{AstronomicalSymbol, EmailSubscription};
use crate::impl_to_from_sql;
use crate::iso_8601::Iso8601;
use diesel::deserialize::FromSqlRow;
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_derive_newtype::DieselNewType;
use std::fmt;
use strum_lite::strum;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct User {
    pub(crate) id: UserId,
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

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub(crate) struct NewUser {
    pub(crate) name: String,
    pub(crate) symbol: AstronomicalSymbol,
    pub(crate) email_subscription: EmailSubscription,
    pub(crate) role: Role,
    pub(crate) email_address: String,
    pub(crate) invited_by: Option<UserId>,
    pub(crate) campaign: Option<String>,
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

strum! {
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, FromSqlRow, AsExpression)]
    #[diesel(sql_type = Text)]
    pub(crate) enum Role {
        Admin = "admin",
        #[default]
        Guest = "guest",
    }
}

impl_to_from_sql! { Role }

use super::{AstronomicalSymbol, EmailSubscription, Role, User, UserId};
use crate::iso_8601::Iso8601;
use diesel::prelude::*;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct UserV2 {
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

impl UserV2 {
    pub(crate) fn to_v1(&self) -> User {
        User {
            id: self.id,
            name: self.name.clone(),
            symbol: self.symbol,
            role: self.role,
            email_address: self.email_address.clone(),
            email_subscription: self.email_subscription,
            invited_by: self.invited_by,
            campaign: self.campaign.clone(),
            can_update_name: self.can_update_name,
            can_answer_strongly: self.can_answer_strongly,
            can_update_symbol: self.can_update_symbol,
            last_active_at: self.last_active_at,
        }
    }
}

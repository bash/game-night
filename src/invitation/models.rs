use super::Passphrase;
use crate::iso_8601::Iso8601;
use crate::users::{Role, UserId};
use diesel::prelude::*;
use diesel_derive_newtype::DieselNewType;
use rand::Rng;
use time::OffsetDateTime;

#[derive(Debug, Copy, Clone, sqlx::Type, DieselNewType)]
#[sqlx(transparent)]
pub(crate) struct InvitationId(pub(crate) i64);

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::invitations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct Invitation {
    pub(crate) id: InvitationId,
    pub(crate) role: Role,
    pub(crate) created_by: Option<UserId>,
    pub(crate) passphrase: Passphrase,
    pub(crate) used_by: Option<UserId>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::invitations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct NewInvitation {
    pub(crate) role: Role,
    pub(crate) created_by: Option<UserId>,
    pub(crate) passphrase: Passphrase,
    pub(crate) comment: String,
    pub(crate) valid_until: Option<Iso8601<OffsetDateTime>>,
}

impl NewInvitation {
    pub(crate) fn builder() -> NewInvitationBuilder {
        NewInvitationBuilder::default()
    }
}

#[derive(Debug, Default)]
pub(crate) struct NewInvitationBuilder {
    role: Role,
    created_by: Option<UserId>,
    valid_until: Option<OffsetDateTime>,
    comment: String,
}

impl NewInvitationBuilder {
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub(crate) fn created_by(mut self, user_id: impl Into<Option<UserId>>) -> Self {
        self.created_by = user_id.into();
        self
    }

    pub(crate) fn valid_until(mut self, valid_until: impl Into<Option<OffsetDateTime>>) -> Self {
        self.valid_until = valid_until.into();
        self
    }

    pub(crate) fn comment(mut self, comment: impl ToString) -> Self {
        self.comment = comment.to_string();
        self
    }

    pub(crate) fn build<R: Rng>(self, rng: &mut R) -> NewInvitation {
        NewInvitation {
            role: self.role,
            created_by: self.created_by,
            valid_until: self.valid_until.map(Iso8601),
            passphrase: rng.random(),
            comment: self.comment,
        }
    }
}

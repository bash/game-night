use super::{Invitation, Passphrase};
use crate::auto_resolve;
use crate::infra::sql_functions::unixepoch;
use crate::infra::DieselConnectionPool;
use crate::iso_8601::Iso8601;
use crate::users::Role;
use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

auto_resolve! {
    pub(crate) struct InvitationQueries {
          connection: DieselConnectionPool,
    }
}

impl InvitationQueries {
    pub(crate) async fn admin(&mut self) -> Result<Option<Invitation>> {
        use crate::schema::invitations::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(invitations
            .filter(role.eq(Role::Admin).and(created_by.is_null()))
            .select(Invitation::as_select())
            .get_result(&mut connection)
            .await
            .optional()?)
    }

    pub(crate) async fn by_passphrase(&mut self, p: &Passphrase) -> Result<Option<Invitation>> {
        use crate::schema::invitations::dsl::*;
        let mut connection = self.connection.get().await?;
        let now = Iso8601(OffsetDateTime::now_utc());
        Ok(invitations
            .filter(
                passphrase.eq(p).and(
                    valid_until
                        .is_null()
                        .or(unixepoch(valid_until.assume_not_null()).ge(unixepoch(now))),
                ),
            )
            .select(Invitation::as_select())
            .get_result(&mut connection)
            .await
            .optional()?)
    }
}

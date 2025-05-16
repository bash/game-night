use super::models::{NewUser, UserV2};
use super::UserId;
use crate::auto_resolve;
use crate::infra::DieselConnectionPool;
use crate::invitation::Invitation;
use crate::iso_8601::Iso8601;
use crate::schema::{invitations, users};
use anyhow::{bail, Error, Result};
use diesel::dsl::*;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use time::OffsetDateTime;

auto_resolve! {
    pub(crate) struct UserCommands {
        connection: DieselConnectionPool,
    }
}

impl UserCommands {
    /// Adds a user while destroying the associated invitation.
    pub(crate) async fn add(&mut self, user: NewUser, invitation: &Invitation) -> Result<UserV2> {
        use crate::schema::invitations::{id as invitation_id, used_by};
        let mut connection = self.connection.get().await?;
        connection
            .transaction(move |connection| {
                Box::pin(async move {
                    let user = insert_into(users::table)
                        .values(&user)
                        .returning(UserV2::as_returning())
                        .get_result(connection)
                        .await?;
                    let updated_rows = update(invitations::table)
                        .filter(invitation_id.eq(invitation.id.0))
                        .set(used_by.eq(user.id))
                        .execute(connection)
                        .await?;
                    if updated_rows == 0 {
                        bail!("invalid invitation with id {id}", id = invitation.id.0)
                    }
                    Ok::<_, Error>(user)
                })
            })
            .await
    }

    pub(crate) async fn remove(&mut self, user_id: UserId) -> Result<()> {
        use crate::schema::users::id;
        let mut connection = self.connection.get().await?;
        delete(users::table)
            .filter(id.eq(user_id.0))
            .execute(&mut connection)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_last_active(
        &mut self,
        user_id: UserId,
        ts: OffsetDateTime,
    ) -> Result<()> {
        use crate::schema::users::{id, last_active_at};
        let mut connection = self.connection.get().await?;
        update(users::table)
            .filter(id.eq(user_id.0).and(last_active_at.lt(Iso8601(ts))))
            .set(last_active_at.eq(Iso8601(ts)))
            .execute(&mut connection)
            .await?;
        Ok(())
    }
}

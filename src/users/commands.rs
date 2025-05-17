use super::models::{NewUser, UserV2};
use super::{AstronomicalSymbol, EmailSubscription, UserId};
use crate::infra::DieselConnectionPool;
use crate::invitation::Invitation;
use crate::iso_8601::Iso8601;
use crate::schema::{invitations, users};
use crate::{auto_resolve, default};
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
                        .filter(invitation_id.eq(invitation.id))
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
        let patch = UserPatch {
            last_active_at: Some(Iso8601(ts)),
            ..default()
        };
        self.update(user_id, patch).await
    }

    pub(crate) async fn update(&mut self, user_id: UserId, patch: UserPatch) -> Result<()> {
        use crate::schema::users::id;
        let mut connection = self.connection.get().await?;
        update(users::table)
            .filter(id.eq(user_id.0))
            .set(patch)
            .execute(&mut connection)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Default, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub(crate) struct UserPatch {
    pub(crate) name: Option<String>,
    pub(crate) symbol: Option<AstronomicalSymbol>,
    pub(crate) email_subscription: Option<EmailSubscription>,
    pub(crate) last_active_at: Option<Iso8601<OffsetDateTime>>,
}

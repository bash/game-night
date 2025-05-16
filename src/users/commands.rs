use super::models::{NewUser, UserV2};
use super::UserId;
use crate::auto_resolve;
use crate::infra::DieselConnectionPool;
use crate::invitation::Invitation;
use crate::schema::{invitations, users};
use anyhow::{bail, Error, Result};
use diesel::{connection, delete, insert_into, update, ExpressionMethods, SelectableHelper};
use diesel_async::{AsyncConnection, RunQueryDsl};

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
}

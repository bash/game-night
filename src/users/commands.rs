use super::models::{NewUser, UserV2};
use crate::auto_resolve;
use crate::infra::DieselConnectionPool;
use crate::invitation::Invitation;
use crate::schema::{invitations, users};
use anyhow::Result;
use diesel::{insert_into, update, ExpressionMethods, SelectableHelper};
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
        Ok(connection
            .transaction(move |connection| {
                Box::pin(async move {
                    let user = insert_into(users::table)
                        .values(&user)
                        .returning(UserV2::as_returning())
                        .get_result(connection)
                        .await?;
                    update(invitations::table)
                        .filter(invitation_id.eq(invitation.id.0))
                        .set(used_by.eq(user.id))
                        .execute(connection)
                        .await?;
                    Ok::<_, diesel::result::Error>(user)
                })
            })
            .await?)
    }
}

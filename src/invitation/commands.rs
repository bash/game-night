use super::{Invitation, NewInvitation};
use crate::auto_resolve;
use crate::infra::DieselConnectionPool;
use anyhow::Result;
use diesel::{insert_into, prelude::*};
use diesel_async::RunQueryDsl;

auto_resolve! {
    pub(crate) struct InvitationCommands {
          connection: DieselConnectionPool,
    }
}

impl InvitationCommands {
    pub(crate) async fn add(&mut self, invitation: NewInvitation) -> Result<Invitation> {
        use crate::schema::invitations::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(insert_into(invitations)
            .values(invitation)
            .returning(Invitation::as_returning())
            .get_result(&mut connection)
            .await?)
    }
}

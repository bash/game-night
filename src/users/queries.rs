use super::models::UserV2;
use super::INACTIVITY_THRESHOLD;
use crate::auth::is_invited_v2;
use crate::auto_resolve;
use crate::event::StatefulEvent;
use crate::infra::sql_functions::unixepoch;
use crate::infra::DieselConnectionPool;
use crate::iso_8601::Iso8601;
use anyhow::Result;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

auto_resolve! {
    pub(crate) struct UserQueries {
        connection: DieselConnectionPool,
    }
}

impl UserQueries {
    pub(crate) async fn all(&mut self) -> Result<Vec<UserV2>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(users
            .select(UserV2::as_select())
            .order_by(last_active_at.desc())
            .load(&mut connection)
            .await?)
    }

    pub(crate) async fn active(&mut self) -> Result<Vec<UserV2>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        let min_active_at = OffsetDateTime::now_utc() - INACTIVITY_THRESHOLD;
        Ok(users
            .filter((unixepoch(last_active_at)).ge(unixepoch(Iso8601(min_active_at))))
            .select(UserV2::as_select())
            .load(&mut connection)
            .await?)
    }

    pub(crate) async fn invited(&mut self, event: &StatefulEvent) -> Result<Vec<UserV2>> {
        let is_invited = |u: &UserV2| is_invited_v2(u, event);
        let users = self.all().await?;
        Ok(users.into_iter().filter(is_invited).collect())
    }

    pub(crate) async fn active_and_invited(
        &mut self,
        event: &StatefulEvent,
    ) -> Result<Vec<UserV2>> {
        let is_invited = |u: &UserV2| is_invited_v2(u, event);
        let active = self.active().await?;
        Ok(active.into_iter().filter(is_invited).collect())
    }
}

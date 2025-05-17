use super::models::User;
use super::{UserId, INACTIVITY_THRESHOLD};
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
    pub(crate) async fn all(&mut self) -> Result<Vec<User>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(users
            .select(User::as_select())
            .order_by(last_active_at.desc())
            .load(&mut connection)
            .await?)
    }

    pub(crate) async fn active(&mut self) -> Result<Vec<User>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        let min_active_at = OffsetDateTime::now_utc() - INACTIVITY_THRESHOLD;
        Ok(users
            .filter((unixepoch(last_active_at)).ge(unixepoch(Iso8601(min_active_at))))
            .select(User::as_select())
            .load(&mut connection)
            .await?)
    }

    pub(crate) async fn invited(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let is_invited = |u: &User| is_invited_v2(u, event);
        let users = self.all().await?;
        Ok(users.into_iter().filter(is_invited).collect())
    }

    pub(crate) async fn active_and_invited(&mut self, event: &StatefulEvent) -> Result<Vec<User>> {
        let is_invited = |u: &User| is_invited_v2(u, event);
        let active = self.active().await?;
        Ok(active.into_iter().filter(is_invited).collect())
    }

    pub(crate) async fn has(&mut self) -> Result<bool> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        let user_count: i64 = users.count().get_result(&mut connection).await?;
        Ok(user_count >= 1)
    }

    pub(crate) async fn by_id(&mut self, user_id: UserId) -> Result<Option<User>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(users
            .filter(id.eq(user_id.0))
            .select(User::as_select())
            .get_result(&mut connection)
            .await
            .optional()?)
    }

    pub(crate) async fn by_id_required(&mut self, user_id: UserId) -> Result<User> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(users
            .filter(id.eq(user_id.0))
            .select(User::as_select())
            .get_result(&mut connection)
            .await?)
    }

    pub(crate) async fn by_email(&mut self, email: &str) -> Result<Option<User>> {
        use crate::schema::users::dsl::*;
        let mut connection = self.connection.get().await?;
        Ok(users
            .filter(email_address.eq(email))
            .select(User::as_select())
            .get_result(&mut connection)
            .await
            .optional()?)
    }
}

use crate::invitation::Invitation;
use crate::users::{User, UserId};
use rocket::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;
use std::error::Error;
use std::ops::DerefMut;

type SqliteConnection = PoolConnection<Sqlite>;

#[async_trait]
pub(crate) trait Repository {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<(), Box<dyn Error>>;

    async fn get_invitation_by_passphrase(
        &mut self,
        passphrase: &str,
    ) -> Result<Option<Invitation>, Box<dyn Error>>;

    /// Adds a user while destroying the associated invitation.
    async fn add_user(
        &mut self,
        invitation: Invitation,
        user: User<()>,
    ) -> Result<UserId, Box<dyn Error>>;

    async fn has_users(&mut self) -> Result<bool, Box<dyn Error>>;
}

pub(crate) struct SqliteRepository(pub(crate) SqliteConnection);

#[async_trait]
impl Repository for SqliteRepository {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<(), Box<dyn Error>> {
        sqlx::query(
            "INSERT INTO invitations (role, created_by, passphrase)
             VALUES (?1, ?2, ?3)",
        )
        .bind(invitation.role)
        .bind(invitation.created_by)
        .bind(invitation.passphrase)
        .execute(self.0.deref_mut())
        .await?;
        Ok(())
    }

    async fn get_invitation_by_passphrase(
        &mut self,
        passphrase: &str,
    ) -> Result<Option<Invitation>, Box<dyn Error>> {
        todo!()
    }

    async fn add_user(
        &mut self,
        invitation: Invitation,
        user: User<()>,
    ) -> Result<UserId, Box<dyn Error>> {
        todo!()
    }

    async fn has_users(&mut self) -> Result<bool, Box<dyn Error>> {
        let user_count: i64 = sqlx::query_scalar("SELECT count(1) as count FROM users")
            .fetch_one(self.0.deref_mut())
            .await?;
        Ok(user_count >= 1)
    }
}

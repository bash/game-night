use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::users::{User, UserId};
use anyhow::Result;
use rocket::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;
use std::ops::DerefMut;

type SqliteConnection = PoolConnection<Sqlite>;

#[async_trait]
pub(crate) trait Repository: Send {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<Invitation>;

    async fn get_invitation_by_passphrase(
        &mut self,
        passphrase: &Passphrase,
    ) -> Result<Option<Invitation>>;

    async fn get_admin_invitation(&mut self) -> Result<Option<Invitation>>;

    /// Adds a user while destroying the associated invitation.
    async fn add_user(&mut self, invitation: Invitation, user: User<()>) -> Result<UserId>;

    async fn has_users(&mut self) -> Result<bool>;
}

pub(crate) struct SqliteRepository(pub(crate) SqliteConnection);

#[async_trait]
impl Repository for SqliteRepository {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<Invitation> {
        let result = sqlx::query(
            "INSERT INTO invitations (role, created_by, passphrase)
             VALUES (?1, ?2, ?3)",
        )
        .bind(invitation.role)
        .bind(invitation.created_by)
        .bind(&invitation.passphrase)
        .execute(self.0.deref_mut())
        .await?;
        Ok(invitation.with_id(InvitationId(result.last_insert_rowid())))
    }

    async fn get_admin_invitation(&mut self) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as(
            "SELECT rowid, * FROM invitations WHERE role = 'admin' AND created_by IS NULL LIMIT 1",
        )
        .fetch_optional(self.0.deref_mut())
        .await?;
        Ok(invitation)
    }

    async fn get_invitation_by_passphrase(
        &mut self,
        passphrase: &Passphrase,
    ) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as("SELECT rowid, * FROM invitations WHERE passphrase = ?1")
            .bind(passphrase)
            .fetch_optional(self.0.deref_mut())
            .await?;
        Ok(invitation)
    }

    async fn add_user(&mut self, invitation: Invitation, user: User<()>) -> Result<UserId> {
        todo!()
    }

    async fn has_users(&mut self) -> Result<bool> {
        let user_count: i64 = sqlx::query_scalar("SELECT count(1) as count FROM users")
            .fetch_one(self.0.deref_mut())
            .await?;
        Ok(user_count >= 1)
    }
}

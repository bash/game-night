use crate::email_verification_code::EmailVerificationCode;
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::users::{User, UserId};
use anyhow::{anyhow, Result};
use chrono::Local;
use rocket::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{Connection, Sqlite};
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

    async fn add_email_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn use_email_verification_code(
        &mut self,
        code: &str,
        email_address: &str,
    ) -> Result<bool>;
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
        let mut transaction = self.0.begin().await?;
        let delete_result = sqlx::query("DELETE FROM invitations WHERE rowid = ?")
            .bind(invitation.id)
            .execute(&mut transaction)
            .await?;
        if delete_result.rows_affected() >= 1 {
            let insert_result = sqlx::query(
                "INSERT INTO users (name, role, email_address, invited_by)
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(user.name)
            .bind(user.role)
            .bind(user.email_address)
            .bind(user.invited_by)
            .execute(&mut transaction)
            .await?;
            transaction.commit().await?;
            Ok(UserId(insert_result.last_insert_rowid()))
        } else {
            transaction.rollback().await?;
            Err(anyhow!("Invalid invitation"))
        }
    }

    async fn has_users(&mut self) -> Result<bool> {
        let user_count: i64 = sqlx::query_scalar("SELECT count(1) as count FROM users")
            .fetch_one(self.0.deref_mut())
            .await?;
        Ok(user_count >= 1)
    }

    async fn add_email_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_verification_codes (code, email_address, valid_until)
             VALUES (?1, ?2, ?3)",
        )
        .bind(&code.code)
        .bind(&code.email_address)
        .bind(code.valid_until)
        .execute(self.0.deref_mut())
        .await?;
        Ok(())
    }

    async fn use_email_verification_code(
        &mut self,
        code: &str,
        email_address: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM email_verification_codes
             WHERE code = ?1
               AND email_address = ?2
               AND valid_until >= ?3",
        )
        .bind(code)
        .bind(email_address)
        .bind(Local::now())
        .execute(self.0.deref_mut())
        .await?;
        Ok(result.rows_affected() >= 1)
    }
}

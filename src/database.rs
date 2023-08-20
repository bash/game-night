use crate::email_verification_code::EmailVerificationCode;
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::login::{LoginToken, LoginTokenType};
use crate::users::{User, UserId};
use crate::GameNightDatabase;
use anyhow::{anyhow, Error, Result};
use chrono::Local;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use rocket_db_pools::Connection;
use sqlx::pool::PoolConnection;
use sqlx::{Connection as _, Sqlite, Transaction};
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

    async fn get_user_by_id(&mut self, user_id: UserId) -> Result<Option<User>>;

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool>;

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool>;

    async fn add_login_token(&mut self, token: LoginToken) -> Result<()>;

    async fn use_login_token(&mut self, token: &str) -> Result<Option<UserId>>;
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
        let user_count: i64 = sqlx::query_scalar("SELECT count(1) FROM users")
            .fetch_one(self.0.deref_mut())
            .await?;
        Ok(user_count >= 1)
    }

    async fn get_user_by_id(&mut self, user_id: UserId) -> Result<Option<User>> {
        let invitation = sqlx::query_as("SELECT rowid, * FROM users WHERE rowid = ?1")
            .bind(user_id)
            .fetch_optional(self.0.deref_mut())
            .await?;
        Ok(invitation)
    }

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()> {
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

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool> {
        let result: i64 = sqlx::query_scalar(
            "SELECT count(1) FROM email_verification_codes
             WHERE email_address = ?1
               AND valid_until >= ?2",
        )
        .bind(email_address)
        .bind(Local::now())
        .fetch_one(self.0.deref_mut())
        .await?;
        Ok(result >= 1)
    }

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool> {
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

    async fn add_login_token(&mut self, token: LoginToken) -> Result<()> {
        sqlx::query(
            "INSERT INTO login_tokens (type, token, user_id, valid_until)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&token.type_)
        .bind(&token.token)
        .bind(&token.user_id)
        .bind(&token.valid_until)
        .execute(self.0.deref_mut())
        .await?;
        Ok(())
    }

    async fn use_login_token(&mut self, token_value: &str) -> Result<Option<UserId>> {
        let mut transaction = self.0.begin().await?;

        let token: Option<LoginToken> =
            sqlx::query_as("SELECT * FROM login_tokens WHERE token = ?1 AND valid_until >= ?2")
                .bind(token_value)
                .bind(Local::now())
                .fetch_optional(&mut transaction)
                .await?;

        if !is_one_time_token(&token) || delete_token(&mut transaction, token_value).await? {
            transaction.commit().await?;
            Ok(token.map(|t| t.user_id))
        } else {
            transaction.rollback().await?;
            Ok(None)
        }
    }
}

async fn delete_token(transaction: &mut Transaction<'_, Sqlite>, token: &str) -> Result<bool> {
    let delete_result = sqlx::query("DELETE FROM login_tokens WHERE token = ?1")
        .bind(token)
        .execute(transaction)
        .await?;
    Ok(delete_result.rows_affected() >= 1)
}

fn is_one_time_token(token: &Option<LoginToken>) -> bool {
    matches!(
        token,
        Some(LoginToken {
            type_: LoginTokenType::OneTime,
            ..
        })
    )
}

#[async_trait]
impl<'r> FromRequest<'r> for Box<dyn Repository> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Connection::<GameNightDatabase>::from_request(request)
            .await
            .map(create_repository)
            .map_failure(|(status, error)| (status, into_anyhow_error(error)))
    }
}

fn into_anyhow_error<E: std::error::Error + Send + Sync + 'static>(error: Option<E>) -> Error {
    error
        .map(Into::into)
        .unwrap_or_else(|| anyhow!("Unable to retrieve database"))
}

fn create_repository(connection: Connection<GameNightDatabase>) -> Box<dyn Repository> {
    Box::new(SqliteRepository(connection.into_inner()))
}

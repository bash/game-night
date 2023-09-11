use crate::email_verification_code::EmailVerificationCode;
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::login::{LoginToken, LoginTokenType};
use crate::poll::{Answer, Poll, PollOption};
use crate::users::{User, UserId};
use crate::GameNightDatabase;
use anyhow::{anyhow, Error, Result};
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use rocket_db_pools::Connection;
use sqlx::pool::PoolConnection;
use sqlx::{Connection as _, Executor, Sqlite};
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

    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>>;

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool>;

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool>;

    async fn has_one_time_login_token(&mut self, email_address: &str) -> Result<bool>;

    async fn add_login_token(&mut self, token: &LoginToken) -> Result<()>;

    async fn use_login_token(&mut self, token: &str) -> Result<Option<UserId>>;

    async fn add_poll(&mut self, poll: Poll<(), UserId>) -> Result<i64>;

    async fn update_poll_description(&mut self, id: i64, description: &str) -> Result<()>;

    async fn add_answer(&mut self, option_id: i64, answer: Answer<(), UserId>) -> Result<()>;

    async fn get_current_poll(&mut self) -> Result<Option<Poll>>;

    async fn close_poll(&mut self, id: i64) -> Result<()>;
}

pub(crate) struct SqliteRepository(pub(crate) SqliteConnection);

#[async_trait]
impl Repository for SqliteRepository {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<Invitation> {
        let result = sqlx::query(
            "INSERT INTO invitations (role, created_by, valid_until, passphrase)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(invitation.role)
        .bind(invitation.created_by)
        .bind(invitation.valid_until)
        .bind(&invitation.passphrase)
        .execute(self.0.deref_mut())
        .await?;
        Ok(invitation.with_id(InvitationId(result.last_insert_rowid())))
    }

    async fn get_admin_invitation(&mut self) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as(
            "SELECT * FROM invitations WHERE role = 'admin' AND created_by IS NULL LIMIT 1",
        )
        .fetch_optional(self.0.deref_mut())
        .await?;
        Ok(invitation)
    }

    async fn get_invitation_by_passphrase(
        &mut self,
        passphrase: &Passphrase,
    ) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as(
            "SELECT * FROM invitations
             WHERE passphrase = ?1
               AND (valid_until IS NULL OR unixepoch(valid_until) - unixepoch('now') >= 0)",
        )
        .bind(passphrase)
        .fetch_optional(self.0.deref_mut())
        .await?;
        Ok(invitation)
    }

    async fn add_user(&mut self, invitation: Invitation, user: User<()>) -> Result<UserId> {
        let mut transaction = self.0.begin().await?;
        let delete_result = sqlx::query("DELETE FROM invitations WHERE id = ?")
            .bind(invitation.id)
            .execute(&mut *transaction)
            .await?;
        if delete_result.rows_affected() >= 1 {
            let insert_result = sqlx::query(
                "INSERT INTO users (name, role, email_address, invited_by, campaign)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(user.name)
            .bind(user.role)
            .bind(user.email_address)
            .bind(user.invited_by)
            .bind(user.campaign)
            .execute(&mut *transaction)
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
        let invitation = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_optional(self.0.deref_mut())
            .await?;
        Ok(invitation)
    }

    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>> {
        let invitation = sqlx::query_as("SELECT * FROM users WHERE email_address = ?1")
            .bind(email)
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
               AND unixepoch(valid_until) - unixepoch('now') >= 0",
        )
        .bind(email_address)
        .fetch_one(self.0.deref_mut())
        .await?;
        Ok(result >= 1)
    }

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM email_verification_codes
             WHERE code = ?1
               AND email_address = ?2
               AND unixepoch(valid_until) - unixepoch('now') >= 0",
        )
        .bind(code)
        .bind(email_address)
        .execute(self.0.deref_mut())
        .await?;
        Ok(result.rows_affected() >= 1)
    }

    async fn add_login_token(&mut self, token: &LoginToken) -> Result<()> {
        sqlx::query(
            "INSERT INTO login_tokens (type, token, user_id, valid_until)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(token.type_)
        .bind(&token.token)
        .bind(token.user_id)
        .bind(token.valid_until)
        .execute(self.0.deref_mut())
        .await?;
        Ok(())
    }

    async fn has_one_time_login_token(&mut self, email_address: &str) -> Result<bool> {
        let token_count: i64 = sqlx::query_scalar(
            "SELECT count(1) FROM login_tokens
             JOIN users ON users.id = login_tokens.user_id
             WHERE users.email_address = ?1
               AND unixepoch(valid_until) - unixepoch('now') >= 0
               AND type = 'one_time'",
        )
        .bind(email_address)
        .fetch_one(self.0.as_mut())
        .await?;
        Ok(token_count >= 1)
    }

    async fn use_login_token(&mut self, token_value: &str) -> Result<Option<UserId>> {
        let mut transaction = self.0.begin().await?;

        let token: Option<LoginToken> =
            sqlx::query_as("SELECT * FROM login_tokens WHERE token = ?1 AND unixepoch(valid_until) - unixepoch('now') >= 0")
                .bind(token_value)
                .fetch_optional(&mut *transaction)
                .await?;

        if !is_one_time_token(&token) || delete_token(&mut *transaction, token_value).await? {
            transaction.commit().await?;
            Ok(token.map(|t| t.user_id))
        } else {
            transaction.rollback().await?;
            Ok(None)
        }
    }

    async fn add_poll(&mut self, poll: Poll<(), UserId>) -> Result<i64> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.0.begin().await?;

        let poll_id = sqlx::query(
            "INSERT INTO polls (min_participants, max_participants, strategy, description, created_by, open_until, closed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(i64::try_from(poll.min_participants)?)
        .bind(i64::try_from(poll.max_participants)?)
        .bind(poll.strategy)
        .bind(poll.description)
        .bind(poll.created_by)
        .bind(poll.open_until)
        .bind(poll.closed)
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        for option in poll.options {
            sqlx::query(
                "INSERT INTO poll_options (poll_id, date, time)
                 VALUES (?1, ?2, ?3)",
            )
            .bind(poll_id)
            .bind(option.date)
            .bind(option.time)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(poll_id)
    }

    async fn update_poll_description(&mut self, id: i64, description: &str) -> Result<()> {
        sqlx::query("UPDATE polls SET description = ?1 WHERE id = ?2")
            .bind(description)
            .bind(id)
            .execute(self.0.deref_mut())
            .await?;
        Ok(())
    }

    async fn get_current_poll(&mut self) -> Result<Option<Poll>> {
        let poll: Option<Poll<i64, UserId>> =
            sqlx::query_as("SELECT * FROM polls ORDER BY polls.open_until DESC LIMIT 1")
                .fetch_optional(self.0.deref_mut())
                .await?;
        match poll {
            Some(poll) => Ok(Some(self.materialize_poll(poll).await?)),
            None => Ok(None),
        }
    }

    async fn add_answer(&mut self, option_id: i64, answer: Answer<(), UserId>) -> Result<()> {
        let mut transaction: sqlx::Transaction<'_, Sqlite> = self.0.begin().await?;

        let closed: bool = sqlx::query_scalar(
            "SELECT polls.closed FROM polls
             JOIN poll_options ON poll_options.poll_id = polls.id
             WHERE poll_options.id = ?1",
        )
        .bind(option_id)
        .fetch_one(&mut *transaction)
        .await?;

        if closed {
            return Err(anyhow!("Poll already closed"));
        }

        sqlx::query(
            "INSERT INTO poll_answers (poll_option_id, value, user_id)
             VALUES (?1, ?2, ?3)",
        )
        .bind(option_id)
        .bind(answer.value)
        .bind(answer.user)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(())
    }

    async fn close_poll(&mut self, id: i64) -> Result<()> {
        sqlx::query("UPDATE polls SET closed = ?1 WHERE id = ?2")
            .bind(true)
            .bind(id)
            .execute(self.0.deref_mut())
            .await?;
        Ok(())
    }
}

impl SqliteRepository {
    async fn materialize_poll(&mut self, poll: Poll<i64, UserId>) -> Result<Poll> {
        // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
        // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(poll.created_by)
            .fetch_one(self.0.deref_mut())
            .await?;

        let mut options: Vec<PollOption> =
            sqlx::query_as("SELECT * FROM poll_options WHERE poll_id = ?1")
                .bind(poll.id)
                .fetch_all(self.0.deref_mut())
                .await?;

        for option in &mut options {
            for answer in sqlx::query_as("SELECT * FROM poll_answers WHERE poll_option_id = ?1")
                .bind(option.id)
                .fetch_all(self.0.deref_mut())
                .await?
            {
                option.answers.push(self.materialize_answer(answer).await?);
            }
        }

        Ok(poll.materialize(user, options))
    }

    async fn materialize_answer(&mut self, answer: Answer<i64, UserId>) -> Result<Answer> {
        let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(answer.user)
            .fetch_one(self.0.deref_mut())
            .await?;
        Ok(answer.materialize(user))
    }
}

async fn delete_token<'c, E>(executor: E, token: &str) -> Result<bool>
where
    E: Executor<'c, Database = Sqlite>,
{
    let delete_result = sqlx::query("DELETE FROM login_tokens WHERE token = ?1")
        .bind(token)
        .execute(executor)
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

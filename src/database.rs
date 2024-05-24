use crate::event::{Event, Participant};
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::login::{LoginToken, LoginTokenType};
use crate::poll::{Answer, Location, Poll, PollOption};
use crate::register::EmailVerificationCode;
use crate::users::{User, UserId, UserPatch};
use crate::GameNightDatabase;
use anyhow::{anyhow, Error, Ok, Result};
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use rocket_db_pools::Connection;
use sqlx::pool::PoolConnection;
use sqlx::{Connection as _, Executor, Sqlite, SqliteConnection};
use std::fmt;
use time::OffsetDateTime;

#[async_trait]
pub(crate) trait Repository: fmt::Debug + Send {
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

    async fn get_users(&mut self) -> Result<Vec<User>>;

    async fn update_user(&mut self, id: UserId, patch: UserPatch) -> Result<()>;

    async fn delete_user(&mut self, id: UserId) -> Result<()>;

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool>;

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool>;

    async fn add_login_token(&mut self, token: &LoginToken) -> Result<()>;

    async fn use_login_token(&mut self, token: &str) -> Result<Option<UserId>>;

    async fn add_poll(&mut self, poll: &Poll<(), UserId, i64>) -> Result<i64>;

    async fn update_poll_open_until(&mut self, id: i64, close_at: OffsetDateTime) -> Result<()>;

    async fn add_answers(&mut self, answers: Vec<(i64, Answer<(), UserId>)>) -> Result<()>;

    async fn get_open_poll(&mut self) -> Result<Option<Poll>>;

    async fn get_polls_pending_for_finalization(&mut self) -> Result<Vec<Poll>>;

    async fn close_poll(&mut self, id: i64) -> Result<()>;

    async fn get_location(&mut self) -> Result<Location>;

    async fn add_event(&mut self, event: Event<(), UserId, i64>) -> Result<Event>;

    async fn get_next_event(&mut self) -> Result<Option<Event>>;

    async fn get_newest_event(&mut self) -> Result<Option<Event>>;

    async fn add_participant(&mut self, event: i64, user: UserId) -> Result<()>;

    async fn prune(&mut self) -> Result<()>;
}

#[derive(Debug)]
pub(crate) struct SqliteRepository(pub(crate) PoolConnection<Sqlite>);

impl SqliteRepository {
    fn executor(&mut self) -> &mut SqliteConnection {
        &mut self.0
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn add_invitation(&mut self, invitation: Invitation<()>) -> Result<Invitation> {
        let result = sqlx::query(
            "INSERT INTO invitations (role, created_by, valid_until, passphrase, comment)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(invitation.role)
        .bind(invitation.created_by)
        .bind(invitation.valid_until)
        .bind(&invitation.passphrase)
        .bind(&invitation.comment)
        .execute(self.executor())
        .await?;
        Ok(invitation.with_id(InvitationId(result.last_insert_rowid())))
    }

    async fn get_admin_invitation(&mut self) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as(
            "SELECT * FROM invitations WHERE role = 'admin' AND created_by IS NULL LIMIT 1",
        )
        .fetch_optional(self.executor())
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
        .fetch_optional(self.executor())
        .await?;
        Ok(invitation)
    }

    async fn add_user(&mut self, invitation: Invitation, user: User<()>) -> Result<UserId> {
        let mut transaction = self.0.begin().await?;

        let user_id = sqlx::query(
            "INSERT INTO users (name, role, email_address, email_subscription, invited_by, campaign)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(user.name)
        .bind(user.role)
        .bind(user.email_address)
        .bind(user.email_subscription)
        .bind(user.invited_by)
        .bind(user.campaign)
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        let update_result =
            sqlx::query("UPDATE invitations SET used_by = ?2 WHERE id = ?1 AND used_by IS NULL")
                .bind(invitation.id)
                .bind(user_id)
                .execute(&mut *transaction)
                .await?;
        if update_result.rows_affected() >= 1 {
            transaction.commit().await?;
            Ok(UserId(user_id))
        } else {
            transaction.rollback().await?;
            Err(anyhow!("Invalid invitation"))
        }
    }

    async fn has_users(&mut self) -> Result<bool> {
        let user_count: i64 = sqlx::query_scalar("SELECT count(1) FROM users")
            .fetch_one(self.executor())
            .await?;
        Ok(user_count >= 1)
    }

    async fn get_user_by_id(&mut self, user_id: UserId) -> Result<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
            .bind(user_id)
            .fetch_optional(self.executor())
            .await?;
        Ok(user)
    }

    async fn get_user_by_email(&mut self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as("SELECT * FROM users WHERE email_address = ?1")
            .bind(email)
            .fetch_optional(self.executor())
            .await?;
        Ok(user)
    }

    async fn get_users(&mut self) -> Result<Vec<User>> {
        Ok(sqlx::query_as("SELECT * FROM users")
            .fetch_all(self.executor())
            .await?)
    }

    async fn update_user(&mut self, id: UserId, patch: UserPatch) -> Result<()> {
        let mut transaction = self.0.begin().await?;
        if let Some(name) = patch.name {
            sqlx::query("UPDATE users SET name = ?2 WHERE id = ?1")
                .bind(id)
                .bind(name)
                .execute(&mut *transaction)
                .await?;
        }
        if let Some(email_subscription) = patch.email_subscription {
            sqlx::query("UPDATE users SET email_subscription = ?2 WHERE id = ?1")
                .bind(id)
                .bind(email_subscription)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn delete_user(&mut self, id: UserId) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = ?1")
            .bind(id)
            .execute(self.executor())
            .await?;
        Ok(())
    }

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()> {
        sqlx::query(
            "INSERT INTO email_verification_codes (code, email_address, valid_until)
             VALUES (?1, ?2, ?3)",
        )
        .bind(&code.code)
        .bind(&code.email_address)
        .bind(code.valid_until)
        .execute(self.executor())
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
        .fetch_one(self.executor())
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
        .execute(self.executor())
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
        .execute(self.executor())
        .await?;
        Ok(())
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

    async fn add_poll(&mut self, poll: &Poll<(), UserId, i64>) -> Result<i64> {
        let mut transaction = self.0.begin().await?;

        let poll_id = sqlx::query(
            "INSERT INTO polls (min_participants, max_participants, strategy, description, location_id, created_by, open_until, closed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(i64::try_from(poll.min_participants)?)
        .bind(i64::try_from(poll.max_participants)?)
        .bind(poll.strategy)
        .bind(&poll.description)
        .bind(poll.location)
        .bind(poll.created_by)
        .bind(poll.open_until)
        .bind(poll.closed)
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        for option in poll.options.iter() {
            let option_id = sqlx::query(
                "INSERT INTO poll_options (poll_id, starts_at, ends_at) VALUES (?1, ?2, ?3)",
            )
            .bind(poll_id)
            .bind(option.starts_at)
            .bind(option.ends_at)
            .execute(&mut *transaction)
            .await?
            .last_insert_rowid();
            for answer in option.answers.iter() {
                sqlx::query(
                    "INSERT INTO poll_answers (poll_option_id, value, user_id)
                     VALUES (?1, ?2, ?3)",
                )
                .bind(option_id)
                .bind(answer.value)
                .bind(answer.user)
                .execute(&mut *transaction)
                .await?;
            }
        }

        transaction.commit().await?;

        Ok(poll_id)
    }

    async fn update_poll_open_until(&mut self, id: i64, open_until: OffsetDateTime) -> Result<()> {
        sqlx::query("UPDATE polls SET open_until = ?1 WHERE id = ?2")
            .bind(open_until)
            .bind(id)
            .execute(self.executor())
            .await?;
        Ok(())
    }

    async fn get_open_poll(&mut self) -> Result<Option<Poll>> {
        let mut transaction = self.0.begin().await?;

        let poll = sqlx::query_as(
            "SELECT * FROM polls
             WHERE unixepoch(open_until) - unixepoch('now') >= 0
               AND closed = ?1
             LIMIT 1",
        )
        .bind(false)
        .fetch_optional(&mut *transaction)
        .await?;
        match poll {
            Some(poll) => Ok(Some(materialize_poll(&mut transaction, poll).await?)),
            None => Ok(None),
        }
    }

    async fn get_polls_pending_for_finalization(&mut self) -> Result<Vec<Poll>> {
        let mut transaction = self.0.begin().await?;

        let polls = sqlx::query_as(
            "SELECT * FROM polls
             WHERE unixepoch(open_until) - unixepoch('now') < 0
               AND closed = ?1
             ORDER BY open_until DESC",
        )
        .bind(false)
        .fetch_all(&mut *transaction)
        .await?;

        let mut materialized_polls = Vec::new();
        for poll in polls {
            materialized_polls.push(materialize_poll(&mut transaction, poll).await?);
        }

        Ok(materialized_polls)
    }

    async fn add_answers(&mut self, answers: Vec<(i64, Answer<(), UserId>)>) -> Result<()> {
        let mut transaction = self.0.begin().await?;

        for (option_id, answer) in answers {
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
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn close_poll(&mut self, id: i64) -> Result<()> {
        sqlx::query("UPDATE polls SET closed = ?1 WHERE id = ?2")
            .bind(true)
            .bind(id)
            .execute(self.executor())
            .await?;
        Ok(())
    }

    async fn get_location(&mut self) -> Result<Location> {
        Ok(sqlx::query_as("SELECT * FROM locations LIMIT 1")
            .fetch_one(self.executor())
            .await?)
    }

    async fn add_event(&mut self, event: Event<(), UserId, i64>) -> Result<Event> {
        let mut transaction = self.0.begin().await?;

        let event_id = sqlx::query(
            "INSERT INTO events (starts_at, ends_at, description, location_id, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(event.starts_at)
        .bind(event.ends_at)
        .bind(&event.description)
        .bind(event.location)
        .bind(event.created_by)
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        for participant in event.participants.iter() {
            sqlx::query("INSERT INTO participants (event_id, user_id) VALUES (?1, ?2)")
                .bind(event_id)
                .bind(participant.user)
                .execute(&mut *transaction)
                .await?;
        }

        let event = materialize_event(&mut transaction, event.with_id(event_id)).await?;

        transaction.commit().await?;

        Ok(event)
    }

    async fn get_next_event(&mut self) -> Result<Option<Event>> {
        let mut transaction = self.0.begin().await?;

        let event: Option<Event<i64, UserId, i64>> = sqlx::query_as(
            "SELECT * FROM events
             WHERE unixepoch(ends_at) - unixepoch('now') >= 0
             ORDER BY starts_at ASC
             LIMIT 1",
        )
        .fetch_optional(&mut *transaction)
        .await?;
        match event {
            None => Ok(None),
            Some(event) => Ok(Some(materialize_event(&mut transaction, event).await?)),
        }
    }

    async fn get_newest_event(&mut self) -> Result<Option<Event>> {
        let mut transaction = self.0.begin().await?;

        let event: Option<Event<i64, UserId, i64>> =
            sqlx::query_as("SELECT * FROM events ORDER BY starts_at DESC LIMIT 1")
                .fetch_optional(&mut *transaction)
                .await?;
        match event {
            None => Ok(None),
            Some(event) => Ok(Some(materialize_event(&mut transaction, event).await?)),
        }
    }

    async fn add_participant(&mut self, event: i64, user: UserId) -> Result<()> {
        sqlx::query("INSERT INTO participants (event_id, user_id) VALUES (?1, ?2)")
            .bind(event)
            .bind(user)
            .execute(self.executor())
            .await?;
        Ok(())
    }

    async fn prune(&mut self) -> Result<()> {
        let mut transaction = self.0.begin().await?;

        sqlx::query("DELETE FROM login_tokens WHERE unixepoch(valid_until) - unixepoch('now') < 0")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM email_verification_codes WHERE unixepoch(valid_until) - unixepoch('now') < 0")
            .execute(&mut *transaction)
            .await?;
        Ok(())
    }
}

async fn materialize_poll(
    connection: &mut SqliteConnection,
    poll: Poll<i64, UserId, i64>,
) -> Result<Poll> {
    // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
    // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(poll.created_by)
        .fetch_one(&mut *connection)
        .await?;

    let location: Location = sqlx::query_as("SELECT * FROM locations WHERE id = ?1")
        .bind(poll.location)
        .fetch_one(&mut *connection)
        .await?;

    let mut options: Vec<PollOption> =
        sqlx::query_as("SELECT * FROM poll_options WHERE poll_id = ?1")
            .bind(poll.id)
            .fetch_all(&mut *connection)
            .await?;

    for option in &mut options {
        for answer in sqlx::query_as("SELECT * FROM poll_answers WHERE poll_option_id = ?1")
            .bind(option.id)
            .fetch_all(&mut *connection)
            .await?
        {
            option
                .answers
                .push(materialize_answer(&mut *connection, answer).await?);
        }
    }

    Ok(poll.materialize(user, location, options))
}

async fn materialize_answer(
    connection: &mut SqliteConnection,
    answer: Answer<i64, UserId>,
) -> Result<Answer> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(answer.user)
        .fetch_one(connection)
        .await?;
    Ok(answer.materialize(user))
}

async fn materialize_event<Participants: Default>(
    connection: &mut SqliteConnection,
    event: Event<i64, UserId, i64, Participants>,
) -> Result<Event> {
    let created_by = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(event.created_by)
        .fetch_one(&mut *connection)
        .await?;
    let location = sqlx::query_as("SELECT * FROM locations WHERE id = ?1")
        .bind(event.location)
        .fetch_one(&mut *connection)
        .await?;
    let participants = sqlx::query_as("SELECT * FROM participants WHERE event_id = ?1")
        .bind(event.id)
        .fetch_all(&mut *connection)
        .await?;
    let participants = materialize_participants(connection, participants).await?;
    Ok(event.materialize(location, created_by, participants))
}

async fn materialize_participants(
    connection: &mut SqliteConnection,
    participants: Vec<Participant<i64, UserId>>,
) -> Result<Vec<Participant>> {
    let mut materialized = Vec::new();
    for participant in participants {
        materialized.push(materialize_participant(&mut *connection, participant).await?);
    }
    Ok(materialized)
}

async fn materialize_participant(
    connection: &mut SqliteConnection,
    participant: Participant<i64, UserId>,
) -> Result<Participant> {
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(participant.user)
        .fetch_one(&mut *connection)
        .await?;
    Ok(participant.materialize(user))
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
            .map(|c| create_repository(c.into_inner()))
            .map_error(|(status, error)| (status, into_anyhow_error(error)))
    }
}

fn into_anyhow_error<E: std::error::Error + Send + Sync + 'static>(error: Option<E>) -> Error {
    error
        .map(Into::into)
        .unwrap_or_else(|| anyhow!("Unable to retrieve database"))
}

pub(crate) fn create_repository(connection: PoolConnection<Sqlite>) -> Box<dyn Repository> {
    Box::new(SqliteRepository(connection))
}

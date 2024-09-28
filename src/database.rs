use crate::email::MessageId;
use crate::event::{
    self, Event, EventEmail, EventId, EventLifecycle, Participant, PlanningDetails, Polling,
};
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::login::{LoginToken, LoginTokenType};
use crate::poll::{Answer, Location, Poll, PollOption, PollStage};
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

mod entity;
pub(crate) use entity::*;

#[async_trait]
pub(crate) trait Repository: EventEmailsRepository + fmt::Debug + Send {
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

    async fn update_last_active(&mut self, id: UserId, ts: OffsetDateTime) -> Result<()>;

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool>;

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool>;

    async fn add_login_token(&mut self, token: &LoginToken) -> Result<()>;

    async fn use_login_token(&mut self, token: &str) -> Result<Option<UserId>>;

    async fn add_poll(&mut self, poll: Poll<New>) -> Result<Poll>;

    async fn update_poll_open_until(&mut self, id: i64, close_at: OffsetDateTime) -> Result<()>;

    async fn add_answers(&mut self, answers: Vec<(i64, Answer<New>)>) -> Result<()>;

    async fn get_open_poll(&mut self) -> Result<Option<Poll>>;

    async fn get_polls_pending_for_finalization(&mut self) -> Result<Vec<Poll>>;

    async fn update_poll_stage(&mut self, id: i64, stage: PollStage) -> Result<()>;

    async fn plan_event(&mut self, id: EventId, details: PlanningDetails) -> Result<Event>;

    async fn get_location(&mut self) -> Result<Location>;

    async fn get_next_event(&mut self) -> Result<Option<Event>>;

    async fn get_newest_event(&mut self) -> Result<Option<Event>>;

    async fn get_events(&mut self) -> Result<Vec<Event>>;

    async fn add_participant(&mut self, event: EventId, user: UserId) -> Result<()>;

    async fn prune(&mut self) -> Result<u64>;

    fn into_event_emails_repository(self: Box<Self>) -> Box<dyn EventEmailsRepository>;
}

#[async_trait]
pub(crate) trait EventEmailsRepository: fmt::Debug + Send {
    async fn add_event_email(&mut self, email: EventEmail) -> Result<()>;

    async fn get_last_message_id(&mut self, event: i64, user: UserId) -> Result<Option<MessageId>>;
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
        let result = sqlx::query!(
            "INSERT INTO invitations (role, created_by, valid_until, passphrase, comment)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            invitation.role,
            invitation.created_by,
            invitation.valid_until,
            invitation.passphrase,
            invitation.comment
        )
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

        let user_id = sqlx::query!(
            "INSERT INTO users (name, role, email_address, email_subscription, invited_by, campaign)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
             user.name,
             user.role,
             user.email_address,
             user.email_subscription,
             user.invited_by,
             user.campaign,
        )
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        let update_result = sqlx::query!(
            "UPDATE invitations SET used_by = ?2 WHERE id = ?1 AND used_by IS NULL",
            invitation.id,
            user_id
        )
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
        let user_count: i64 = sqlx::query_scalar!("SELECT count(1) FROM users")
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
        Ok(
            sqlx::query_as("SELECT * FROM users ORDER BY last_active_at DESC")
                .fetch_all(self.executor())
                .await?,
        )
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

    async fn update_last_active(&mut self, id: UserId, ts: OffsetDateTime) -> Result<()> {
        sqlx::query("UPDATE users SET last_active_at = max(last_active_at, ?1) WHERE id = ?2")
            .bind(&ts)
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
        let result: i64 = sqlx::query_scalar!(
            "SELECT count(1) FROM email_verification_codes
             WHERE email_address = ?1
               AND unixepoch(valid_until) - unixepoch('now') >= 0",
            email_address
        )
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

    async fn add_poll(&mut self, poll: Poll<New>) -> Result<Poll> {
        let mut transaction = self.0.begin().await?;

        let event_id = sqlx::query!(
            "INSERT INTO events (title, description, location_id, created_by)
             VALUES (?1, ?2, ?3, ?4)",
            poll.event.title,
            poll.event.description,
            poll.event.location,
            poll.event.created_by
        )
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        let min_participants = i64::try_from(poll.min_participants)?;
        let max_participants = i64::try_from(poll.max_participants)?;
        let poll_id = sqlx::query!(
            "INSERT INTO polls (min_participants, max_participants, strategy, open_until, stage, event_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
             min_participants,
             max_participants,
             poll.strategy,
             poll.open_until,
             poll.stage,
             event_id
        )
        .execute(&mut *transaction)
        .await?
        .last_insert_rowid();

        for option in poll.options.iter() {
            let option_id = sqlx::query!(
                "INSERT INTO poll_options (poll_id, starts_at) VALUES (?1, ?2)",
                poll_id,
                option.starts_at
            )
            .execute(&mut *transaction)
            .await?
            .last_insert_rowid();
            for answer in option.answers.iter() {
                sqlx::query!(
                    "INSERT INTO poll_answers (poll_option_id, value, user_id)
                     VALUES (?1, ?2, ?3)",
                    option_id,
                    answer.value,
                    answer.user
                )
                .execute(&mut *transaction)
                .await?;
            }
        }

        let poll = poll.into_unmaterialized(poll_id, event_id);
        let poll = materialize_poll(&mut *transaction, poll).await?;

        transaction.commit().await?;

        Ok(poll)
    }

    async fn update_poll_open_until(&mut self, id: i64, open_until: OffsetDateTime) -> Result<()> {
        sqlx::query!(
            "UPDATE polls SET open_until = ?1 WHERE id = ?2",
            open_until,
            id
        )
        .execute(self.executor())
        .await?;
        Ok(())
    }

    async fn get_open_poll(&mut self) -> Result<Option<Poll>> {
        let mut transaction = self.0.begin().await?;

        let poll = sqlx::query_as(
            "SELECT * FROM polls
             WHERE unixepoch(open_until) - unixepoch('now') >= 0
               AND stage = ?1
             LIMIT 1",
        )
        .bind(PollStage::Open)
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
               AND stage = ?1
             ORDER BY open_until DESC",
        )
        .bind(PollStage::Open)
        .fetch_all(&mut *transaction)
        .await?;

        let mut materialized_polls = Vec::new();
        for poll in polls {
            materialized_polls.push(materialize_poll(&mut transaction, poll).await?);
        }

        Ok(materialized_polls)
    }

    async fn add_answers(&mut self, answers: Vec<(i64, Answer<New>)>) -> Result<()> {
        let mut transaction = self.0.begin().await?;

        for (option_id, answer) in answers {
            let stage: PollStage = sqlx::query_scalar!(
                r#"SELECT stage as "stage: _" FROM polls
                 JOIN poll_options ON poll_options.poll_id = polls.id
                 WHERE poll_options.id = ?1"#,
                option_id
            )
            .fetch_one(&mut *transaction)
            .await?;

            if stage != PollStage::Open {
                return Err(anyhow!("Poll already closed"));
            }

            sqlx::query!(
                "INSERT INTO poll_answers (poll_option_id, value, user_id)
                 VALUES (?1, ?2, ?3)",
                option_id,
                answer.value,
                answer.user,
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn update_poll_stage(&mut self, id: i64, stage: PollStage) -> Result<()> {
        // TODO: merge this with plan_event
        sqlx::query!("UPDATE polls SET stage = ?1 WHERE id = ?2", stage, id)
            .execute(self.executor())
            .await?;
        Ok(())
    }

    async fn plan_event(&mut self, id: EventId, details: PlanningDetails) -> Result<Event> {
        let mut transaction = self.0.begin().await?;
        sqlx::query!(
            "UPDATE events SET starts_at = ?2 WHERE id = ?1",
            id,
            details.starts_at
        )
        .execute(&mut *transaction)
        .await?;
        for participant in details.participants.iter() {
            sqlx::query!(
                "INSERT INTO participants (event_id, user_id) VALUES (?1, ?2)",
                id,
                participant.user
            )
            .execute(&mut *transaction)
            .await?;
        }

        let event: Event<_> = sqlx::query_as("SELECT * FROM events WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *transaction)
            .await?;
        let event = materialize_event(&mut transaction, event).await?;

        transaction.commit().await?;

        Ok(event)
    }

    async fn get_location(&mut self) -> Result<Location> {
        Ok(sqlx::query_as("SELECT * FROM locations LIMIT 1")
            .fetch_one(self.executor())
            .await?)
    }

    async fn get_next_event(&mut self) -> Result<Option<Event>> {
        let mut transaction = self.0.begin().await?;

        let event: Option<Event<Unmaterialized>> = sqlx::query_as(
            "SELECT * FROM events
             WHERE (unixepoch(starts_at) + ?1) - unixepoch('now') >= 0
             ORDER BY starts_at ASC
             LIMIT 1",
        )
        .bind(event::ESTIMATED_DURATION.whole_seconds())
        .fetch_optional(&mut *transaction)
        .await?;
        match event {
            None => Ok(None),
            Some(event) => Ok(Some(materialize_event(&mut transaction, event).await?)),
        }
    }

    async fn get_newest_event(&mut self) -> Result<Option<Event>> {
        let mut transaction = self.0.begin().await?;

        let event: Option<Event<Unmaterialized>> =
            sqlx::query_as("SELECT * FROM events ORDER BY starts_at DESC LIMIT 1")
                .fetch_optional(&mut *transaction)
                .await?;
        match event {
            None => Ok(None),
            Some(event) => Ok(Some(materialize_event(&mut transaction, event).await?)),
        }
    }

    async fn get_events(&mut self) -> Result<Vec<Event>> {
        let mut transaction = self.0.begin().await?;
        let events: Vec<Event<Unmaterialized>> =
            sqlx::query_as("SELECT * FROM events ORDER BY starts_at DESC")
                .fetch_all(&mut *transaction)
                .await?;
        let mut materialized: Vec<_> = Vec::with_capacity(events.len());
        for event in events {
            materialized.push(materialize_event(&mut transaction, event).await?);
        }
        Ok(materialized)
    }

    async fn add_participant(&mut self, event: EventId, user: UserId) -> Result<()> {
        sqlx::query!(
            "INSERT INTO participants (event_id, user_id) VALUES (?1, ?2)",
            event,
            user
        )
        .execute(self.executor())
        .await?;
        Ok(())
    }

    async fn prune(&mut self) -> Result<u64> {
        let mut transaction = self.0.begin().await?;

        let tokens_result = sqlx::query!(
            "DELETE FROM login_tokens WHERE unixepoch(valid_until) - unixepoch('now') < 0",
        )
        .execute(&mut *transaction)
        .await?;
        let codes_result = sqlx::query!("DELETE FROM email_verification_codes WHERE unixepoch(valid_until) - unixepoch('now') < 0")
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(tokens_result.rows_affected() + codes_result.rows_affected())
    }

    fn into_event_emails_repository(self: Box<Self>) -> Box<dyn EventEmailsRepository> {
        Box::new(*self)
    }
}

#[async_trait]
impl EventEmailsRepository for SqliteRepository {
    async fn add_event_email(&mut self, email: EventEmail) -> Result<()> {
        sqlx::query!(
            "INSERT INTO event_emails (event_id, user_id, message_id, subject) VALUES (?1, ?2, ?3, ?4)",
            email.event,
            email.user,
            email.message_id,
            email.subject)
        .execute(self.executor()).await?;
        Ok(())
    }

    async fn get_last_message_id(&mut self, event: i64, user: UserId) -> Result<Option<MessageId>> {
        Ok(sqlx::query_scalar!(
            r#"SELECT message_id as "message_id: _" FROM event_emails
               WHERE event_id = ?1 AND user_id = ?2
               ORDER BY created_at DESC LIMIT 1"#,
            event,
            user
        )
        .fetch_optional(self.executor())
        .await?)
    }
}

async fn materialize_poll(
    connection: &mut SqliteConnection,
    poll: Poll<Unmaterialized>,
) -> Result<Poll> {
    // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
    // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
    let event: Event<Unmaterialized, Polling> =
        sqlx::query_as("SELECT * FROM events WHERE id = ?1")
            .bind(poll.event)
            .fetch_one(&mut *connection)
            .await?;
    let event = materialize_event(connection, event).await?;

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

    Ok(poll.into_materialized(event, options))
}

async fn materialize_answer(
    connection: &mut SqliteConnection,
    answer: Answer<Unmaterialized>,
) -> Result<Answer> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(answer.user)
        .fetch_one(connection)
        .await?;
    Ok(answer.materialize(user))
}

async fn materialize_event<L: EventLifecycle>(
    connection: &mut SqliteConnection,
    event: Event<Unmaterialized, L>,
) -> Result<Event<Materialized, L>> {
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
    Ok(event.into_materialized(location, created_by, participants))
}

async fn materialize_participants(
    connection: &mut SqliteConnection,
    participants: Vec<Participant<Unmaterialized>>,
) -> Result<Vec<Participant>> {
    let mut materialized = Vec::new();
    for participant in participants {
        materialized.push(materialize_participant(&mut *connection, participant).await?);
    }
    Ok(materialized)
}

async fn materialize_participant(
    connection: &mut SqliteConnection,
    participant: Participant<Unmaterialized>,
) -> Result<Participant> {
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(participant.user)
        .fetch_one(&mut *connection)
        .await?;
    Ok(participant.into_materialized(user))
}

async fn delete_token<'c, E>(executor: E, token: &str) -> Result<bool>
where
    E: Executor<'c, Database = Sqlite>,
{
    let delete_result = sqlx::query!("DELETE FROM login_tokens WHERE token = ?1", token)
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

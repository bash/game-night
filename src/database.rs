use crate::email::MessageId;
use crate::event::{
    Event, EventEmail, EventId, EventLifecycle, Location, Organizer, Participant, PlanningDetails,
    Polling, StatefulEvent,
};
use crate::groups::Group;
use crate::invitation::{Invitation, InvitationId, Passphrase};
use crate::login::{LoginToken, LoginTokenType};
use crate::poll::{Answer, Poll, PollOption, PollOptionPatch, PollStage};
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

    async fn get_active_users(&mut self, inactivity_threshold: time::Duration)
        -> Result<Vec<User>>;

    async fn get_groups(&mut self) -> Result<Vec<Group>>;

    async fn update_user(&mut self, id: UserId, patch: UserPatch) -> Result<()>;

    async fn delete_user(&mut self, id: UserId) -> Result<()>;

    async fn update_last_active(&mut self, id: UserId, ts: OffsetDateTime) -> Result<()>;

    async fn add_verification_code(&mut self, code: &EmailVerificationCode) -> Result<()>;

    async fn has_verification_code(&mut self, email_address: &str) -> Result<bool>;

    async fn use_verification_code(&mut self, code: &str, email_address: &str) -> Result<bool>;

    async fn add_login_token(&mut self, token: &LoginToken) -> Result<()>;

    async fn use_login_token(&mut self, token: &str) -> Result<Option<UserId>>;

    async fn add_poll(&mut self, poll: Poll<New>) -> Result<Poll>;

    async fn add_answers(&mut self, answers: Vec<(i64, Answer<New>)>) -> Result<()>;

    async fn update_poll_option(
        &mut self,
        poll_option_id: i64,
        patch: PollOptionPatch,
    ) -> Result<()>;

    async fn add_event(&mut self, event: Event<New, Polling>) -> Result<i64>;

    async fn update_poll_stage(&mut self, id: i64, stage: PollStage) -> Result<()>;

    async fn plan_event(&mut self, id: EventId, details: PlanningDetails) -> Result<Event>;

    async fn get_stateful_event(&mut self, id: EventId) -> Result<Option<StatefulEvent>>;

    async fn get_stateful_events(&mut self) -> Result<Vec<StatefulEvent>>;

    async fn get_location_by_id(&mut self, id: i64) -> Result<Option<Location>>;

    async fn get_locations(&mut self) -> Result<Vec<Location>>;

    async fn add_participant(&mut self, event: EventId, user: UserId) -> Result<()>;

    async fn remove_participant(&mut self, event: EventId, user: UserId) -> Result<()>;

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
            "INSERT INTO users (name, symbol, role, email_address, email_subscription, invited_by, campaign)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
             user.name,
             user.symbol,
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

    async fn get_active_users(
        &mut self,
        inactivity_threshold: time::Duration,
    ) -> Result<Vec<User>> {
        let min_active_at = OffsetDateTime::now_utc() - inactivity_threshold;
        Ok(sqlx::query_as(
            "SELECT * FROM users
             WHERE unixepoch(last_active_at) - unixepoch(?1) >= 0
             ORDER BY last_active_at DESC",
        )
        .bind(min_active_at)
        .fetch_all(self.executor())
        .await?)
    }

    async fn get_groups(&mut self) -> Result<Vec<Group>> {
        let mut transaction = self.0.begin().await?;
        let groups = sqlx::query_as("SELECT * FROM groups")
            .fetch_all(&mut *transaction)
            .await?;
        let mut materialized = Vec::with_capacity(groups.len());
        for group in groups {
            materialized.push(materialize_group(&mut transaction, group).await?);
        }
        Ok(materialized)
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
        if let Some(symbol) = patch.symbol {
            sqlx::query("UPDATE users SET symbol = ?2 WHERE id = ?1")
                .bind(id)
                .bind(symbol)
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
            .bind(ts)
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

        let event_id = insert_event(&mut transaction, &poll.event).await?;

        let min_participants = i64::try_from(poll.min_participants)?;
        let poll_id = sqlx::query!(
            "INSERT INTO polls (min_participants, strategy, open_until, stage, event_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            min_participants,
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
                "INSERT INTO poll_options (poll_id, starts_at, promote) VALUES (?1, ?2, ?3)",
                poll_id,
                option.starts_at,
                option.promote,
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
        let poll = materialize_poll(&mut transaction, poll, None).await?;

        transaction.commit().await?;

        Ok(poll)
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

            if !stage.accepts_answers() {
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

    async fn update_poll_option(
        &mut self,
        poll_option_id: i64,
        patch: PollOptionPatch,
    ) -> Result<()> {
        let mut transaction = self.0.begin().await?;
        if let Some(promote) = patch.promote {
            sqlx::query("UPDATE poll_options SET promote = ?2 WHERE id = ?1")
                .bind(poll_option_id)
                .bind(promote)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn add_event(&mut self, event: Event<New, Polling>) -> Result<i64> {
        let mut transaction = self.0.begin().await?;
        let id = insert_event(&mut transaction, &event).await?;
        transaction.commit().await?;
        Ok(id)
    }

    async fn update_poll_stage(&mut self, id: i64, stage: PollStage) -> Result<()> {
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

    async fn get_stateful_event(&mut self, id: EventId) -> Result<Option<StatefulEvent>> {
        let mut transaction = self.0.begin().await?;
        let event = sqlx::query_as("SELECT * FROM events WHERE id = ?1 LIMIT 1")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?;
        Ok(match event {
            Some(event) => {
                let now = OffsetDateTime::now_utc();
                Some(materialize_stateful_event(&mut transaction, event, now).await?)
            }
            None => None,
        })
    }

    async fn get_stateful_events(&mut self) -> Result<Vec<StatefulEvent>> {
        let mut transaction = self.0.begin().await?;
        let events = sqlx::query_as("SELECT * FROM events")
            .fetch_all(&mut *transaction)
            .await?;
        let mut materialized = Vec::with_capacity(events.len());
        let now = OffsetDateTime::now_utc();
        for event in events {
            let event = materialize_stateful_event(&mut transaction, event, now).await?;
            materialized.push(event);
        }
        Ok(materialized)
    }

    async fn get_location_by_id(&mut self, id: i64) -> Result<Option<Location>> {
        let mut transaction = self.0.begin().await?;
        let Some(location) = sqlx::query_as("SELECT * FROM locations WHERE id = ?1")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(
            materialize_location(&mut transaction, location).await?,
        ))
    }

    async fn get_locations(&mut self) -> Result<Vec<Location>> {
        let mut transaction = self.0.begin().await?;
        let locations = sqlx::query_as("SELECT * FROM locations")
            .fetch_all(&mut *transaction)
            .await?;
        let mut materialized = Vec::with_capacity(locations.len());
        for location in locations {
            let location = materialize_location(&mut transaction, location).await?;
            materialized.push(location);
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

    async fn remove_participant(&mut self, event: EventId, user: UserId) -> Result<()> {
        sqlx::query!(
            "DELETE FROM participants WHERE event_id = ?1 AND user_id = ?2",
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
    event: Option<Event<Materialized, Polling>>,
) -> Result<Poll> {
    // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
    // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
    let event = if let Some(event) = event {
        event
    } else {
        let event = sqlx::query_as("SELECT * FROM events WHERE id = ?1")
            .bind(poll.event)
            .fetch_one(&mut *connection)
            .await?;
        materialize_event(connection, event).await?
    };

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

async fn materialize_stateful_event(
    connection: &mut SqliteConnection,
    event: Event<Unmaterialized, Polling>,
    now: OffsetDateTime,
) -> Result<StatefulEvent> {
    let event = materialize_event(connection, event).await?;
    if let Some(starts_at) = event.starts_at {
        Ok(StatefulEvent::from_planned(event, starts_at, now))
    } else {
        // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
        // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
        let poll = sqlx::query_as("SELECT * FROM polls WHERE id = ?1")
            .bind(event.id)
            .fetch_one(&mut *connection)
            .await?;
        let poll = materialize_poll(connection, poll, Some(event)).await?;
        Ok(StatefulEvent::from_polling(poll, now))
    }
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
    let location = materialize_location(connection, location).await?;
    let participants = sqlx::query_as("SELECT * FROM participants WHERE event_id = ?1")
        .bind(event.id)
        .fetch_all(&mut *connection)
        .await?;
    let participants = materialize_participants(connection, participants).await?;
    let restrict_to = if let Some(restrict_to) = event.restrict_to {
        let group = sqlx::query_as("SELECT * FROM groups WHERE id = ?1")
            .bind(restrict_to)
            .fetch_one(&mut *connection)
            .await?;
        Some(materialize_group(connection, group).await?)
    } else {
        None
    };
    Ok(event.into_materialized(location, created_by, participants, restrict_to))
}

async fn materialize_group(
    connection: &mut SqliteConnection,
    group: Group<Unmaterialized>,
) -> Result<Group> {
    let members = sqlx::query_as(
        "SELECT users.* FROM members
         JOIN users ON users.id = members.user_id
         WHERE members.group_id = ?1",
    )
    .bind(group.id)
    .fetch_all(&mut *connection)
    .await?;
    Ok(group.into_materialized(members))
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

async fn materialize_location(
    connection: &mut SqliteConnection,
    location: Location<Unmaterialized>,
) -> Result<Location> {
    let organizers = sqlx::query_as("SELECT * FROM organizers WHERE location_id = ?1")
        .bind(location.id)
        .fetch_all(&mut *connection)
        .await?;
    let organizers = materialize_organizers(connection, organizers).await?;
    Ok(location.into_materialized(organizers))
}

async fn materialize_organizers(
    connection: &mut SqliteConnection,
    organizers: Vec<Organizer<Unmaterialized>>,
) -> Result<Vec<Organizer>> {
    let mut materialized = Vec::new();
    for organizer in organizers {
        materialized.push(materialize_organizer(&mut *connection, organizer).await?);
    }
    Ok(materialized)
}

async fn materialize_organizer(
    connection: &mut SqliteConnection,
    organizer: Organizer<Unmaterialized>,
) -> Result<Organizer> {
    let user = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(organizer.user)
        .fetch_one(&mut *connection)
        .await?;
    Ok(organizer.into_materialized(user))
}

async fn insert_event<L>(connection: &mut SqliteConnection, event: &Event<New, L>) -> Result<i64>
where
    L: EventLifecycle,
{
    let event_id = sqlx::query!(
        "INSERT INTO events (title, description, location_id, created_by, restrict_to, parent_id, cancelled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        event.title,
        event.description,
        event.location,
        event.created_by,
        event.restrict_to,
        event.parent_id,
        event.cancelled,
    )
    .execute(&mut *connection)
    .await?
    .last_insert_rowid();
    Ok(event_id)
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

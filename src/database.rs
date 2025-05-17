use crate::email::MessageId;
use crate::event::{
    Event, EventEmail, EventId, EventLifecycle, Location, Organizer, Participant, PlanningDetails,
    Polling, StatefulEvent,
};
use crate::groups::Group;
use crate::impl_from_request_for_service;
use crate::login::{LoginToken, LoginTokenType};
use crate::poll::{Answer, Poll, PollOption, PollOptionPatch, PollStage};
use crate::push::PushSubscription;
use crate::register::EmailVerificationCode;
use crate::services::{Resolve, ResolveContext};
use crate::users::{UserId, UserQueries};
use anyhow::{anyhow, Context as _, Ok, Result};
use rocket::async_trait;
use rocket_db_pools::{Database, Pool as _};
use sqlx::pool::PoolConnection;
use sqlx::{Connection as _, Executor, Sqlite, SqliteConnection, SqlitePool};
use std::fmt;
use time::OffsetDateTime;

mod entity;
pub(crate) use entity::*;
use nameof::name_of;

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

#[async_trait]
pub(crate) trait Repository: EventEmailsRepository + fmt::Debug + Send {
    async fn get_groups(&mut self) -> Result<Vec<Group>>;

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

    async fn update_poll_stage(&mut self, id: EventId, stage: PollStage) -> Result<()>;

    async fn plan_event(&mut self, id: EventId, details: PlanningDetails) -> Result<Event>;

    async fn get_stateful_event(&mut self, id: EventId) -> Result<Option<StatefulEvent>>;

    async fn get_stateful_events(&mut self) -> Result<Vec<StatefulEvent>>;

    async fn get_location_by_id(&mut self, id: i64) -> Result<Option<Location>>;

    async fn get_locations(&mut self) -> Result<Vec<Location>>;

    async fn add_participant(&mut self, event: EventId, user: UserId) -> Result<()>;

    async fn remove_participant(&mut self, event: EventId, user: UserId) -> Result<()>;

    async fn add_push_subscription(&mut self, subscription: PushSubscription<New>) -> Result<()>;

    async fn remove_push_subscription(&mut self, user_id: UserId, endpoint: &str) -> Result<()>;

    async fn get_push_subscriptions(&mut self, user_id: UserId) -> Result<Vec<PushSubscription>>;

    async fn has_push_subscription(&mut self, user_id: UserId) -> Result<bool>;

    async fn prune(&mut self) -> Result<u64>;

    fn into_event_emails_repository(self: Box<Self>) -> Box<dyn EventEmailsRepository>;
}

#[async_trait]
pub(crate) trait EventEmailsRepository: fmt::Debug + Send {
    async fn add_event_email(&mut self, email: EventEmail) -> Result<()>;

    async fn get_last_message_id(&mut self, event: i64, user: UserId) -> Result<Option<MessageId>>;
}

pub(crate) struct SqliteRepository(PoolConnection<Sqlite>, UserQueries);

impl fmt::Debug for SqliteRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(name_of!(SqliteRepository))
            .finish_non_exhaustive()
    }
}

impl SqliteRepository {
    fn executor(&mut self) -> &mut SqliteConnection {
        &mut self.0
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn get_groups(&mut self) -> Result<Vec<Group>> {
        let mut transaction = self.0.begin().await?;
        let groups = sqlx::query_as("SELECT * FROM groups")
            .fetch_all(&mut *transaction)
            .await?;
        let mut materialized = Vec::with_capacity(groups.len());
        for group in groups {
            materialized.push(materialize_group(&mut transaction, &mut self.1, group).await?);
        }
        Ok(materialized)
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
        let poll = materialize_poll(&mut transaction, &mut self.1, poll, None).await?;

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
        sqlx::query!("UPDATE polls SET stage = ?1 WHERE event_id = ?2", stage, id)
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
        let event = materialize_event(&mut transaction, &mut self.1, event).await?;

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
                Some(materialize_stateful_event(&mut transaction, &mut self.1, event, now).await?)
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
            let event =
                materialize_stateful_event(&mut transaction, &mut self.1, event, now).await?;
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
            materialize_location(&mut transaction, &mut self.1, location).await?,
        ))
    }

    async fn get_locations(&mut self) -> Result<Vec<Location>> {
        let mut transaction = self.0.begin().await?;
        let locations = sqlx::query_as("SELECT * FROM locations")
            .fetch_all(&mut *transaction)
            .await?;
        let mut materialized = Vec::with_capacity(locations.len());
        for location in locations {
            let location = materialize_location(&mut transaction, &mut self.1, location).await?;
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

    async fn add_push_subscription(&mut self, subscription: PushSubscription<New>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO web_push_subscriptions (endpoint, keys, user_id) VALUES (?1, ?2, ?3)",
            subscription.endpoint,
            subscription.keys,
            subscription.user_id,
        )
        .execute(self.executor())
        .await?;
        Ok(())
    }

    async fn remove_push_subscription(&mut self, user_id: UserId, endpoint: &str) -> Result<()> {
        sqlx::query!(
            "DELETE FROM web_push_subscriptions WHERE user_id = ?1 AND endpoint = ?2",
            user_id,
            endpoint,
        )
        .execute(self.executor())
        .await?;
        Ok(())
    }

    async fn get_push_subscriptions(&mut self, user_id: UserId) -> Result<Vec<PushSubscription>> {
        Ok(
            sqlx::query_as("SELECT * FROM web_push_subscriptions WHERE user_id = ?1")
                .bind(user_id)
                .fetch_all(self.executor())
                .await?,
        )
    }

    async fn has_push_subscription(&mut self, user_id: UserId) -> Result<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT count(1) FROM web_push_subscriptions WHERE user_id = ?1")
                .bind(user_id)
                .fetch_one(self.executor())
                .await?;
        Ok(count >= 1)
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
    users: &mut UserQueries,
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
        materialize_event(connection, users, event).await?
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
                .push(materialize_answer(users, answer).await?);
        }
    }

    Ok(poll.into_materialized(event, options))
}

async fn materialize_answer(
    users: &mut UserQueries,
    answer: Answer<Unmaterialized>,
) -> Result<Answer> {
    let user = users.by_id_required(answer.user).await?;
    Ok(answer.materialize(user.to_v1()))
}

async fn materialize_stateful_event(
    connection: &mut SqliteConnection,
    users: &mut UserQueries,
    event: Event<Unmaterialized, Polling>,
    now: OffsetDateTime,
) -> Result<StatefulEvent> {
    let event = materialize_event(connection, users, event).await?;
    if let Some(starts_at) = event.starts_at {
        Ok(StatefulEvent::from_planned(event, starts_at, now))
    } else {
        // Yes, yes using a JOIN to fetch the poll and the user at once would be better,
        // but it's very inconvenient as I can't use the auto-derived FromRow impl :/
        let poll = sqlx::query_as("SELECT * FROM polls WHERE event_id = ?1")
            .bind(event.id)
            .fetch_one(&mut *connection)
            .await?;
        let poll = materialize_poll(connection, users, poll, Some(event)).await?;
        Ok(StatefulEvent::from_polling(poll, now))
    }
}

async fn materialize_event<L: EventLifecycle>(
    connection: &mut SqliteConnection,
    users: &mut UserQueries,
    event: Event<Unmaterialized, L>,
) -> Result<Event<Materialized, L>> {
    let created_by = users.by_id_required(event.created_by).await?;
    let location = sqlx::query_as("SELECT * FROM locations WHERE id = ?1")
        .bind(event.location)
        .fetch_one(&mut *connection)
        .await?;
    let location = materialize_location(connection, users, location).await?;
    let participants = sqlx::query_as("SELECT * FROM participants WHERE event_id = ?1")
        .bind(event.id)
        .fetch_all(&mut *connection)
        .await?;
    let participants = materialize_participants(users, participants).await?;
    let restrict_to = if let Some(restrict_to) = event.restrict_to {
        let group = sqlx::query_as("SELECT * FROM groups WHERE id = ?1")
            .bind(restrict_to)
            .fetch_one(&mut *connection)
            .await?;
        Some(materialize_group(connection, users, group).await?)
    } else {
        None
    };
    Ok(event.into_materialized(location, created_by.to_v1(), participants, restrict_to))
}

async fn materialize_group(
    connection: &mut SqliteConnection,
    users: &mut UserQueries,
    group: Group<Unmaterialized>,
) -> Result<Group> {
    let user_ids: Vec<(UserId,)> = sqlx::query_as(
        "SELECT members.user_id FROM members
         WHERE members.group_id = ?1",
    )
    .bind(group.id)
    .fetch_all(&mut *connection)
    .await?;
    let mut members = Vec::new();
    for (user_id,) in user_ids {
        members.push(users.by_id_required(user_id).await?.to_v1());
    }
    Ok(group.into_materialized(members))
}

async fn materialize_participants(
    users: &mut UserQueries,
    participants: Vec<Participant<Unmaterialized>>,
) -> Result<Vec<Participant>> {
    let mut materialized = Vec::new();
    for participant in participants {
        materialized.push(materialize_participant(users, participant).await?);
    }
    Ok(materialized)
}

async fn materialize_participant(
    users: &mut UserQueries,
    participant: Participant<Unmaterialized>,
) -> Result<Participant> {
    let user = users.by_id_required(participant.user).await?;
    Ok(participant.into_materialized(user.to_v1()))
}

async fn materialize_location(
    connection: &mut SqliteConnection,
    users: &mut UserQueries,
    location: Location<Unmaterialized>,
) -> Result<Location> {
    let organizers = sqlx::query_as("SELECT * FROM organizers WHERE location_id = ?1")
        .bind(location.id)
        .fetch_all(&mut *connection)
        .await?;
    let organizers = materialize_organizers(users, organizers).await?;
    Ok(location.into_materialized(organizers))
}

async fn materialize_organizers(
    users: &mut UserQueries,
    organizers: Vec<Organizer<Unmaterialized>>,
) -> Result<Vec<Organizer>> {
    let mut materialized = Vec::new();
    for organizer in organizers {
        materialized.push(materialize_organizer(users, organizer).await?);
    }
    Ok(materialized)
}

async fn materialize_organizer(
    users: &mut UserQueries,
    organizer: Organizer<Unmaterialized>,
) -> Result<Organizer> {
    let user = users.by_id_required(organizer.user).await?;
    Ok(organizer.into_materialized(user.to_v1()))
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

impl Resolve for Box<dyn Repository> {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        let db = GameNightDatabase::fetch(ctx.rocket()).context("unable to retrieve database")?;
        let connection = db.get().await?;
        Ok(Box::new(SqliteRepository(connection, ctx.resolve().await?)))
    }
}

impl_from_request_for_service!(Box<dyn Repository>);

impl Resolve for Box<dyn EventEmailsRepository> {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        ctx.resolve::<Box<dyn Repository>>()
            .await
            .map(|r| r.into_event_emails_repository())
    }
}

impl_from_request_for_service!(Box<dyn EventEmailsRepository>);

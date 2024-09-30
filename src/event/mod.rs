use crate::database::{Materialized, New, Unmaterialized};
use crate::entity_state;
use crate::iso_8601::Iso8601;
use crate::poll::{Location, PollOption};
use crate::users::{User, UserId};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

mod email;
pub(crate) use email::*;
mod stateful;
pub(crate) use stateful::*;
mod page;
pub(crate) use page::*;
mod participants;
pub(crate) use participants::*;

pub type EventId = i64;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct Event<S: EventState = Materialized, L: EventLifecycle = Planned> {
    pub(crate) id: S::Id,
    pub(crate) starts_at: L::StartsAt,
    pub(crate) title: String,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: S::Location,
    pub(crate) created_by: S::CreatedBy,
    #[sqlx(skip)]
    pub(crate) participants: S::Participants,
}

#[derive(Debug)]
pub(crate) struct PlanningDetails {
    pub(crate) starts_at: <Planned as EventLifecycle>::StartsAt,
    pub(crate) participants: Vec<Participant<New>>,
}

impl PlanningDetails {
    pub(crate) fn new(chosen_option: &PollOption, invited: &[User]) -> Self {
        Self {
            starts_at: chosen_option.starts_at,
            participants: invited
                .iter()
                .map(|u| Participant { id: (), user: u.id })
                .collect(),
        }
    }
}

entity_state! {
    pub(crate) trait EventState {
        type Id = () => EventId => EventId;
        type CreatedBy = UserId => UserId => User;
        type Location = i64 => i64 => Location;
        type Participants: Default = Vec<Participant<Self>> => () => Vec<Participant<Self>>;
    }
}

pub(crate) trait EventLifecycle {
    type StartsAt: Send + Sync;
}

#[derive(Debug, Clone)]
pub(crate) struct Polling;

impl EventLifecycle for Polling {
    type StartsAt = Option<Iso8601<OffsetDateTime>>;
}

#[derive(Debug, Clone)]
pub(crate) struct Planned;

impl EventLifecycle for Planned {
    type StartsAt = Iso8601<OffsetDateTime>;
}

pub(crate) const ESTIMATED_DURATION: Duration = Duration::hours(4);

impl<S: EventState> Event<S> {
    pub(crate) fn estimated_ends_at(&self) -> OffsetDateTime {
        self.starts_at
            .checked_add(ESTIMATED_DURATION)
            .expect("no overflow")
    }
}

impl<L: EventLifecycle> Event<Unmaterialized, L> {
    pub(crate) fn into_materialized(
        self,
        location: Location,
        created_by: User,
        participants: Vec<Participant>,
    ) -> Event<Materialized, L> {
        Event {
            id: self.id,
            starts_at: self.starts_at,
            title: self.title,
            description: self.description,
            location,
            created_by,
            participants,
        }
    }
}

impl<S: EventState> Event<S, Polling> {
    pub(crate) fn into_planned(
        self,
        starts_at: <Planned as EventLifecycle>::StartsAt,
    ) -> Event<S, Planned> {
        Event {
            id: self.id,
            starts_at,
            title: self.title,
            description: self.description,
            location: self.location,
            created_by: self.created_by,
            participants: self.participants,
        }
    }
}

impl Event {
    pub(crate) fn is_participant(&self, user: &User) -> bool {
        self.participants.iter().any(|p| p.user.id == user.id)
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct Participant<S: ParticipantState = Materialized> {
    pub(crate) id: S::Id,
    #[sqlx(rename = "user_id")]
    pub(crate) user: S::User,
}

entity_state! {
    pub(crate) trait ParticipantState {
        type Id = () => i64 => i64;
        type User = UserId => UserId => User;
    }
}

impl Participant<Unmaterialized> {
    pub(crate) fn into_materialized(self, user: User) -> Participant {
        Participant { id: self.id, user }
    }
}

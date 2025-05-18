use crate::database::{Materialized, New, Unmaterialized};
use crate::entity_state;
use crate::iso_8601::Iso8601;
use crate::locations::{Location, Organizer};
use crate::poll::PollOption;
use crate::users::{User, UserId};
use time::{Duration, OffsetDateTime};

mod email;
pub(crate) use email::*;
mod stateful;
pub(crate) use stateful::*;
mod page;
pub(crate) use page::*;
mod participants;
pub(crate) use participants::*;
mod query;
pub(crate) use query::*;
mod view_model;
pub(crate) use view_model::*;
mod ics_file;
use crate::groups::Group;
pub(crate) use ics_file::*;
mod leave;
pub(crate) use leave::*;
mod list;
pub(crate) use list::*;
mod title;
pub(crate) use title::*;
mod convert;
pub(crate) use convert::*;

// TODO: strong type
pub type EventId = i64;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Event<S: EventState = Materialized, L: EventLifecycle = Planned> {
    pub(crate) id: S::Id,
    pub(crate) starts_at: L::StartsAt,
    // TODO: make title an option with a non-empty string.
    pub(crate) title: String,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: S::Location,
    pub(crate) created_by: S::CreatedBy,
    pub(crate) restrict_to: Option<S::RestrictTo>,
    pub(crate) cancelled: bool,
    pub(crate) parent_id: Option<i64>,
    #[sqlx(skip)]
    pub(crate) participants: S::Participants,
}

#[derive(Debug, Clone)]
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
        type RestrictTo = i64 => i64 => Group;
    }
}

pub(crate) trait EventLifecycle {
    type StartsAt: Send + Sync + Copy;
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
        restrict_to: Option<Group>,
    ) -> Event<Materialized, L> {
        Event {
            id: self.id,
            starts_at: self.starts_at,
            title: self.title,
            description: self.description,
            cancelled: self.cancelled,
            parent_id: self.parent_id,
            location,
            created_by,
            participants,
            restrict_to,
        }
    }
}

impl<L: EventLifecycle> Event<Materialized, L> {
    pub(crate) fn to_new(&self) -> Event<New, L> {
        Event {
            id: (),
            starts_at: self.starts_at,
            title: self.title.clone(),
            description: self.description.clone(),
            cancelled: self.cancelled,
            parent_id: self.parent_id,
            location: self.location.id,
            created_by: self.created_by.id,
            participants: Vec::default(),
            restrict_to: self.restrict_to.as_ref().map(|u| u.id),
        }
    }
}

impl<L: EventLifecycle> Event<Materialized, L> {
    pub(crate) fn has_organizer(&self, user: &User) -> bool {
        let is_same_user = |o: &Organizer| o.user.id == user.id;
        self.location.organizers.iter().any(is_same_user)
    }

    pub(crate) fn has_organizer_v2(&self, user: &User) -> bool {
        let is_same_user = |o: &Organizer| o.user.id == user.id;
        self.location.organizers.iter().any(is_same_user)
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
            cancelled: self.cancelled,
            participants: self.participants,
            restrict_to: self.restrict_to,
            parent_id: self.parent_id,
        }
    }
}

impl Event {
    pub(crate) fn is_participant(&self, user: &User) -> bool {
        self.participants.iter().any(|p| p.user.id == user.id)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
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

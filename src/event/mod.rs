use crate::database::{Materialized, New, Unmaterialized};
use crate::entity_state;
use crate::iso_8601::Iso8601;
use crate::poll::{Location, Poll, PollOption};
use crate::users::{User, UserId};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Event<S: EventState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) starts_at: Iso8601<OffsetDateTime>,
    pub(crate) title: String,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: S::Location,
    pub(crate) created_by: S::CreatedBy,
    #[sqlx(skip)]
    pub(crate) participants: S::Participants,
}

entity_state! {
    pub(crate) trait EventState {
        type Id = () => i64 => i64;
        type CreatedBy = UserId => UserId => User;
        type Location = i64 => i64 => Location;
        type Participants: Default = Vec<Participant<Self>> => () => Vec<Participant<Self>>;
    }
}

impl Event<New> {
    pub(crate) fn new(poll: &Poll, chosen_option: &PollOption, participants: &[User]) -> Self {
        Self {
            id: (),
            starts_at: chosen_option.starts_at,
            title: poll.title.clone(),
            description: poll.description.clone(),
            location: poll.location.id,
            created_by: poll.created_by.id,
            participants: participants
                .iter()
                .map(|user| Participant {
                    id: (),
                    user: user.id,
                })
                .collect(),
        }
    }
}

impl Event<New> {
    pub(crate) fn into_unmaterialized(self, id: i64) -> Event<Unmaterialized> {
        Event {
            id,
            starts_at: self.starts_at,
            title: self.title,
            description: self.description,
            location: self.location,
            created_by: self.created_by,
            participants: (),
        }
    }
}

pub(crate) const ESTIMATED_DURATION: Duration = Duration::hours(4);

impl<S: EventState> Event<S> {
    pub(crate) fn estimated_ends_at(&self) -> OffsetDateTime {
        self.starts_at
            .checked_add(ESTIMATED_DURATION)
            .expect("no overflow")
    }
}

impl Event<Unmaterialized> {
    pub(crate) fn into_materialized(
        self,
        location: Location,
        created_by: User,
        participants: Vec<Participant>,
    ) -> Event {
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

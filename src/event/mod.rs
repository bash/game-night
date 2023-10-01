use crate::poll::{Location, Poll, PollOption};
use crate::users::{User, UserId};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Event<Id = i64, UserRef = User, LocationRef = Location> {
    pub(crate) id: Id,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) starts_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) ends_at: OffsetDateTime,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: LocationRef,
    pub(crate) created_by: UserRef,
    #[sqlx(skip)]
    pub(crate) participants: Vec<Participant<Id, UserRef>>,
}

impl Event<(), UserId, i64> {
    pub(crate) fn new(poll: &Poll, chosen_option: &PollOption, participants: &[User]) -> Self {
        Self {
            id: (),
            starts_at: chosen_option.starts_at,
            ends_at: chosen_option.ends_at,
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

impl Event<i64, UserId, i64> {
    pub(crate) fn materialize(
        self,
        location: Location,
        created_by: User,
        participants: Vec<Participant>,
    ) -> Event {
        Event {
            id: self.id,
            starts_at: self.starts_at,
            ends_at: self.ends_at,
            description: self.description,
            location,
            created_by,
            participants,
        }
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Participant<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    #[sqlx(rename = "user_id")]
    pub(crate) user: UserRef,
}

impl Participant<i64, UserId> {
    pub(crate) fn materialize(self, user: User) -> Participant {
        Participant { id: self.id, user }
    }
}

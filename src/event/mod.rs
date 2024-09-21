use crate::iso_8601::Iso8601;
use crate::poll::{Location, Poll, PollOption};
use crate::users::{User, UserId};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Event<
    Id = i64,
    UserRef = User,
    LocationRef = Location,
    Participants = Vec<Participant<Id, UserRef>>,
> where
    Participants: Default,
{
    pub(crate) id: Id,
    pub(crate) starts_at: Iso8601<OffsetDateTime>,
    pub(crate) title: String,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: LocationRef,
    pub(crate) created_by: UserRef,
    #[sqlx(skip)]
    pub(crate) participants: Participants,
}

impl Event<(), UserId, i64> {
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

impl<UserRef, LocationRef, Participants: Default> Event<(), UserRef, LocationRef, Participants> {
    pub(crate) fn with_id(self, id: i64) -> Event<i64, UserRef, LocationRef, Participants> {
        Event {
            id,
            starts_at: self.starts_at,
            title: self.title,
            description: self.description,
            location: self.location,
            created_by: self.created_by,
            participants: self.participants,
        }
    }
}

pub(crate) const ESTIMATED_DURATION: Duration = Duration::hours(4);

impl<Id, UserRef, LocationRef, Participants: Default>
    Event<Id, UserRef, LocationRef, Participants>
{
    pub(crate) fn estimated_ends_at(&self) -> OffsetDateTime {
        self.starts_at
            .checked_add(ESTIMATED_DURATION)
            .expect("no overflow")
    }
}

impl<Participants: Default> Event<i64, UserId, i64, Participants> {
    pub(crate) fn materialize(
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

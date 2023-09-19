use serde::Serialize;
use time::OffsetDateTime;

use crate::poll::{Location, Poll, PollOption};
use crate::users::{User, UserId};

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Event<Id = i64, UserRef = User, LocationRef = Location> {
    pub(crate) id: Id,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) datetime: OffsetDateTime,
    pub(crate) description: String,
    #[sqlx(rename = "location_id")]
    pub(crate) location: LocationRef,
    pub(crate) created_by: UserRef,
    #[sqlx(skip)]
    pub(crate) participants: Vec<Participant<Id, UserRef>>,
}

impl Event<(), UserId, i64> {
    pub(crate) fn new(poll: &Poll, chosen_option: &PollOption, participants: &[UserId]) -> Self {
        Self {
            id: (),
            datetime: chosen_option.datetime,
            description: poll.description.clone(),
            location: poll.location.id,
            created_by: poll.created_by.id,
            participants: participants
                .iter()
                .copied()
                .map(|user| Participant { id: (), user })
                .collect(),
        }
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Participant<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    #[sqlx(rename = "user_id")]
    pub(crate) user: UserRef,
}

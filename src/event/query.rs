use super::{ActiveEvent, Event, EventId, StatefulEvent};
use crate::users::Role;
use crate::{database::Repository, users::User};
use anyhow::Result;
use itertools::Itertools;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use std::cmp::Reverse;

#[derive(Debug)]
pub(crate) struct EventsQuery {
    repository: Box<dyn Repository>,
}

impl EventsQuery {
    /// Fetches all active events (i.e. non-archived and non-failed).
    pub(crate) async fn active(&mut self, user: &User) -> Result<Vec<ActiveEvent>> {
        Ok(self
            .all(user)
            .await?
            .into_iter()
            .filter_map(|e| ActiveEvent::try_from(e).ok())
            .collect())
    }

    /// Fetches all events.
    pub(crate) async fn all(&mut self, user: &User) -> Result<Vec<StatefulEvent>> {
        self.repository
            .get_stateful_events()
            .await
            .map(|e| e.into_iter().filter(is_invited(user)).collect())
    }

    /// Fetches an event for the given id.
    pub(crate) async fn with_id(
        &mut self,
        id: EventId,
        user: &User,
    ) -> Result<Option<StatefulEvent>> {
        self.repository
            .get_stateful_event(id)
            .await
            .map(|e| e.filter(is_invited(user)))
    }

    pub(crate) async fn newest(&mut self, user: &User) -> Result<Option<Event>> {
        Ok(self
            .all(user)
            .await?
            .into_iter()
            .filter_map(|e| Event::try_from(e).ok())
            .sorted_by_key(|e| Reverse(e.starts_at.0))
            .next())
    }
}

fn is_invited<'a>(user: &'a User) -> impl Fn(&StatefulEvent) -> bool + 'a {
    |event| {
        let group = event.restrict_to();
        group.is_none()
            || user.role == Role::Admin
            || group.is_some_and(|group| group.has_member(user))
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for EventsQuery {
    type Error = <Box<dyn Repository> as FromRequest<'r>>::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let repository = try_outcome!(request.guard().await);
        Outcome::Success(EventsQuery { repository })
    }
}

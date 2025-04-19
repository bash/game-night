use super::{ActiveEvent, Event, EventId, StatefulEvent};
use crate::auth::is_invited;
use crate::auto_resolve;
use crate::{database::Repository, users::User};
use anyhow::Result;
use itertools::Itertools;
use std::cmp::Reverse;

auto_resolve! {
    #[derive(Debug)]
    pub(crate) struct EventsQuery {
        repository: Box<dyn Repository>,
    }
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
            .map(|e| e.into_iter().filter(|e| is_invited(user, e)).collect())
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
            .map(|e| e.filter(|e| is_invited(user, e)))
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

use super::{Event, StatefulEvent, VisibleParticipants};
use crate::users::User;
use rocket::{http::uri::Origin, uri};
use std::ops;

#[derive(Debug)]
pub(crate) struct EventViewModel {
    event: StatefulEvent,
    pub(crate) view_uri: Option<Origin<'static>>,
    pub(crate) visible_participants: Option<VisibleParticipants>,
}

impl ops::Deref for EventViewModel {
    type Target = StatefulEvent;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl EventViewModel {
    pub(crate) fn from_event(event: impl Into<StatefulEvent>, user: &User) -> Self {
        let event = event.into();
        let is_active = event.is_active();
        let visible_participants = Event::try_from(event.clone())
            .map(|e| VisibleParticipants::from_event(&e, user, is_active))
            .ok();
        Self {
            view_uri: event.id().map(|id| uri!(crate::event::event_page(id))),
            event,
            visible_participants,
        }
    }
}

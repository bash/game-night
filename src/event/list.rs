use super::{EventViewModel, StatefulEvent};
use crate::template_v2::prelude::*;
use crate::users::UserNameComponent;
use time::OffsetDateTime;

#[derive(Debug, Template)]
#[template(path = "event/event-list.html")]
pub(crate) struct EventListComponent<'a> {
    events: &'a [EventViewModel],
    show_year: bool,
}

impl<'a> EventListComponent<'a> {
    pub(crate) fn for_events(events: &'a [EventViewModel]) -> Self {
        Self {
            events,
            show_year: false,
        }
    }

    pub(crate) fn show_year(mut self, show_year: bool) -> Self {
        self.show_year = show_year;
        self
    }

    fn date_format(&self) -> &'static str {
        if self.show_year {
            "[day]. [month repr:long] [year]"
        } else {
            "[day]. [month repr:long]"
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum EventLabel {
    Date(OffsetDateTime),
    OpenUntil(OffsetDateTime),
    ClosedAt(OffsetDateTime),
}

impl EventLabel {
    fn for_event(event: &StatefulEvent) -> Self {
        use EventLabel::*;
        use StatefulEvent::*;
        match event {
            Polling(poll) => OpenUntil(poll.open_until.0),
            Pending(poll) | Finalizing(poll) | Failed(poll) => ClosedAt(poll.open_until.0),
            Planned(event) | Cancelled(event) | Archived(event) => Date(event.starts_at.0),
        }
    }
}

#[derive(Debug, Template)]
#[template(path = "event/event-list-badge.html")]
struct EventBadgeComponent<'a> {
    event: &'a EventViewModel,
}

impl<'a> EventBadgeComponent<'a> {
    fn for_event(event: &'a EventViewModel) -> Self {
        Self { event }
    }
}

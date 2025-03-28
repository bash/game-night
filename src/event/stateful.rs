use super::{Event, EventId, Planned, Polling};
use crate::database::Materialized;
use crate::groups::Group;
use crate::iso_8601::Iso8601;
use crate::poll::{Poll, PollStage};
use crate::users::User;
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum StatefulEvent {
    /// Poll is open.
    Polling(Poll),
    /// Waiting for finalization.
    Pending(Poll),
    /// Currently in the process of being finalized.
    Finalizing(Poll),
    /// Event is planned or currently ongoing.
    Planned(Event),
    /// An event that was cancelled.
    Cancelled(Event),
    /// Event is in the past.
    Archived(Event),
    /// No poll option had enough votes.
    Failed(Poll),
}

/// An event or poll that is not archived or failed.
#[derive(Debug, Clone)]
pub(crate) enum ActiveEvent {
    Polling(Poll),
    Pending(Poll),
    Finalizing(Poll),
    Planned(Event),
    Cancelled(Event),
}

impl StatefulEvent {
    pub(crate) fn from_poll(poll: Poll, now: OffsetDateTime) -> Self {
        if let Some(starts_at) = poll.event.starts_at {
            Self::from_planned(poll.event, starts_at, now)
        } else {
            Self::from_polling(poll, now)
        }
    }

    pub(crate) fn from_planned(
        event: Event<Materialized, Polling>,
        starts_at: Iso8601<OffsetDateTime>,
        now: OffsetDateTime,
    ) -> StatefulEvent {
        let event = event.into_planned(starts_at);

        // Archived takes precedence over cancelled so that
        // the event eventually becomes inactive.
        if now <= event.estimated_ends_at() {
            if event.cancelled {
                StatefulEvent::Cancelled(event)
            } else {
                StatefulEvent::Planned(event)
            }
        } else {
            StatefulEvent::Archived(event)
        }
    }

    pub(crate) fn from_polling(poll: Poll, now: OffsetDateTime) -> Self {
        match poll.stage {
            PollStage::Open if now <= poll.open_until.0 => StatefulEvent::Polling(poll),
            PollStage::Blocked => StatefulEvent::Polling(poll),
            PollStage::Open | PollStage::Pending => StatefulEvent::Pending(poll),
            PollStage::Finalizing => StatefulEvent::Finalizing(poll),
            PollStage::Closed => StatefulEvent::Failed(poll),
        }
    }

    pub(crate) fn date(&self) -> OffsetDateTime {
        use StatefulEvent::*;
        match self {
            Polling(poll) | Pending(poll) | Finalizing(poll) | Failed(poll) => poll.open_until.0,
            Planned(event) | Cancelled(event) | Archived(event) => event.starts_at.0,
        }
    }

    pub(crate) fn id(&self) -> Option<EventId> {
        use StatefulEvent::*;
        match self {
            Polling(poll) | Pending(poll) | Finalizing(poll) => Some(poll.event.id),
            Planned(event) | Cancelled(event) | Archived(event) => Some(event.id),
            Failed(_) => None,
        }
    }

    pub(crate) fn restrict_to(&self) -> Option<&Group> {
        self.visit_event(|e| e.restrict_to.as_ref(), |e| e.restrict_to.as_ref())
    }

    pub(crate) fn has_organizer(&self, user: &User) -> bool {
        self.visit_event(|e| e.has_organizer(user), |e| e.has_organizer(user))
    }

    // This should match the TryFrom impl for ActiveEvent
    pub(crate) fn is_active(&self) -> bool {
        use StatefulEvent::*;
        matches!(self, Polling(_) | Pending(_) | Finalizing(_) | Planned(_))
    }

    pub(crate) fn polling(self) -> Option<Poll> {
        if let StatefulEvent::Polling(poll) = self {
            Some(poll)
        } else {
            None
        }
    }

    pub(crate) fn pending(self) -> Option<Poll> {
        if let StatefulEvent::Pending(poll) = self {
            Some(poll)
        } else {
            None
        }
    }

    fn visit_event<'a, R>(
        &'a self,
        polling: impl FnOnce(&'a Event<Materialized, Polling>) -> R,
        planned: impl FnOnce(&'a Event<Materialized, Planned>) -> R,
    ) -> R {
        use StatefulEvent::*;
        match self {
            Polling(poll) | Pending(poll) | Finalizing(poll) | Failed(poll) => polling(&poll.event),
            Planned(event) | Cancelled(event) | Archived(event) => planned(event),
        }
    }
}

impl From<ActiveEvent> for StatefulEvent {
    fn from(value: ActiveEvent) -> Self {
        match value {
            ActiveEvent::Polling(poll) => StatefulEvent::Polling(poll),
            ActiveEvent::Pending(poll) => StatefulEvent::Pending(poll),
            ActiveEvent::Finalizing(poll) => StatefulEvent::Finalizing(poll),
            ActiveEvent::Planned(event) => StatefulEvent::Planned(event),
            ActiveEvent::Cancelled(event) => StatefulEvent::Cancelled(event),
        }
    }
}

impl TryFrom<StatefulEvent> for ActiveEvent {
    type Error = ();

    fn try_from(value: StatefulEvent) -> Result<Self, Self::Error> {
        use StatefulEvent::*;
        match value {
            Polling(poll) => Ok(ActiveEvent::Polling(poll)),
            Pending(poll) => Ok(ActiveEvent::Pending(poll)),
            Finalizing(poll) => Ok(ActiveEvent::Finalizing(poll)),
            Planned(event) => Ok(ActiveEvent::Planned(event)),
            Cancelled(event) => Ok(ActiveEvent::Cancelled(event)),
            Archived(_) | Failed(_) => Err(()),
        }
    }
}

impl TryFrom<StatefulEvent> for Event {
    type Error = Poll;

    fn try_from(value: StatefulEvent) -> Result<Self, Self::Error> {
        use StatefulEvent::*;
        match value {
            Planned(event) | Cancelled(event) | Archived(event) => Ok(event),
            Polling(poll) | Pending(poll) | Finalizing(poll) | Failed(poll) => Err(poll),
        }
    }
}

impl ActiveEvent {
    pub(crate) fn event_id(&self) -> EventId {
        use ActiveEvent::*;
        match self {
            Polling(poll) | Pending(poll) | Finalizing(poll) => poll.event.id,
            Planned(event) | Cancelled(event) => event.id,
        }
    }

    pub(crate) fn date(&self) -> OffsetDateTime {
        use ActiveEvent::*;
        match self {
            Polling(poll) | Pending(poll) | Finalizing(poll) => poll.open_until.0,
            Planned(event) | Cancelled(event) => event.starts_at.0,
        }
    }
}

use super::{Event, EventId};
use crate::iso_8601::Iso8601;
use crate::poll::{Poll, PollStage};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum StatefulEvent {
    Polling(Poll),
    Finalizing(Poll),
    Planned(Event),
    Archived(Event),
    Failed(Poll),
}

/// An event or poll that is not archived or failed.
#[derive(Debug, Clone)]
pub(crate) enum ActiveEvent {
    Polling(Poll),
    Finalizing(Poll),
    Planned(Event),
}

impl StatefulEvent {
    pub(crate) fn from_poll(poll: Poll, now: OffsetDateTime) -> Self {
        if let Some(starts_at) = poll.event.starts_at {
            from_planned(poll, starts_at, now)
        } else {
            from_polling(poll, now)
        }
    }

    pub(crate) fn date(&self) -> OffsetDateTime {
        use StatefulEvent::*;
        match self {
            Polling(poll) | Finalizing(poll) | Failed(poll) => poll.open_until.0,
            Planned(event) | Archived(event) => event.starts_at.0,
        }
    }

    pub(crate) fn id(&self) -> Option<EventId> {
        use StatefulEvent::*;
        match self {
            Polling(poll) | Finalizing(poll) => Some(poll.event.id),
            Planned(event) | Archived(event) => Some(event.id),
            Failed(_) => None,
        }
    }

    // This should match the TryFrom impl for ActiveEvent
    pub(crate) fn is_active(&self) -> bool {
        use StatefulEvent::*;
        matches!(self, Polling(_) | Finalizing(_) | Planned(_))
    }
}

impl From<ActiveEvent> for StatefulEvent {
    fn from(value: ActiveEvent) -> Self {
        match value {
            ActiveEvent::Polling(poll) => StatefulEvent::Polling(poll),
            ActiveEvent::Finalizing(poll) => StatefulEvent::Finalizing(poll),
            ActiveEvent::Planned(event) => StatefulEvent::Planned(event),
        }
    }
}

impl TryFrom<StatefulEvent> for ActiveEvent {
    type Error = ();

    fn try_from(value: StatefulEvent) -> Result<Self, Self::Error> {
        use StatefulEvent::*;
        match value {
            Polling(poll) => Ok(ActiveEvent::Polling(poll)),
            Finalizing(poll) => Ok(ActiveEvent::Finalizing(poll)),
            Planned(event) => Ok(ActiveEvent::Planned(event)),
            Archived(_) | Failed(_) => Err(()),
        }
    }
}

impl TryFrom<StatefulEvent> for Event {
    type Error = Poll;

    fn try_from(value: StatefulEvent) -> Result<Self, Self::Error> {
        use StatefulEvent::*;
        match value {
            Planned(event) | Archived(event) => Ok(event),
            Polling(poll) | Finalizing(poll) | Failed(poll) => Err(poll),
        }
    }
}

impl ActiveEvent {
    pub(crate) fn event_id(&self) -> EventId {
        use ActiveEvent::*;
        match self {
            Polling(poll) | Finalizing(poll) => poll.event.id,
            Planned(event) => event.id,
        }
    }

    pub(crate) fn date(&self) -> OffsetDateTime {
        use ActiveEvent::*;
        match self {
            Polling(poll) | Finalizing(poll) => poll.open_until.0,
            Planned(event) => event.starts_at.0,
        }
    }
}

fn from_polling(poll: Poll, now: OffsetDateTime) -> StatefulEvent {
    match poll.stage {
        PollStage::Open if now <= poll.open_until.0 => StatefulEvent::Polling(poll),
        PollStage::Open | PollStage::Finalizing => StatefulEvent::Finalizing(poll),
        PollStage::Closed => StatefulEvent::Failed(poll),
    }
}

fn from_planned(
    poll: Poll,
    starts_at: Iso8601<OffsetDateTime>,
    now: OffsetDateTime,
) -> StatefulEvent {
    let event = poll.event.into_planned(starts_at);
    if now <= event.estimated_ends_at() {
        StatefulEvent::Planned(event)
    } else {
        StatefulEvent::Archived(event)
    }
}

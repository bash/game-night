use super::{Event, EventId};
use crate::iso_8601::Iso8601;
use crate::poll::{Poll, PollStage};
use serde::Serialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize)]
pub(crate) enum StatefulEvent {
    Polling(Poll),
    Finalizing(Poll),
    Planned(Event),
    Archived(Event),
    Failed(Poll),
}

/// An event or poll that is not archived or failed.
#[derive(Debug, Clone, Serialize)]
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
    type Error = ();

    fn try_from(value: StatefulEvent) -> Result<Self, Self::Error> {
        use StatefulEvent::*;
        match value {
            Planned(event) | Archived(event) => Ok(event),
            Polling(_) | Finalizing(_) | Failed(_) => Err(()),
        }
    }
}

impl ActiveEvent {
    pub(crate) fn event_id(&self) -> EventId {
        match self {
            ActiveEvent::Polling(poll) | ActiveEvent::Finalizing(poll) => poll.event.id,
            ActiveEvent::Planned(event) => event.id,
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

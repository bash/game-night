use super::Event;
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

impl StatefulEvent {
    pub(crate) fn from_poll(poll: Poll, now: OffsetDateTime) -> Self {
        if let Some(starts_at) = poll.event.starts_at {
            from_planned(poll, starts_at, now)
        } else {
            from_polling(poll, now)
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

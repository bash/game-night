use crate::{
    database::Repository,
    event::{Event, Participant},
    play::{is_participating, rocket_uri_macro_play_page, NextEvent},
    template::PageBuilder,
    uri,
    users::{Role, User, UserId},
};
use anyhow::Error;
use itertools::Itertools;
use rocket::{get, http::uri::Origin, response::Debug};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[get("/archive")]
pub(crate) async fn archive_page(
    user: User,
    next_event: Option<NextEvent>,
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
) -> Result<Template, Debug<Error>> {
    let events = repository.get_events().await?;
    let events_by_year: Vec<_> = events
        .into_iter()
        .chunk_by(|e| e.starts_at.year())
        .into_iter()
        .map(|(year, events)| Year {
            year,
            events: events
                .map(|e| to_event_view(e, &user, next_event.as_ref()))
                .collect(),
        })
        .collect();
    Ok(page.render("play/archive", context! { events_by_year }))
}

#[derive(Debug, Serialize)]
struct Year {
    year: i32,
    events: Vec<EventView>,
}

#[derive(Debug, Serialize)]
struct EventView {
    #[serde(flatten)]
    event: Event,
    view_uri: Option<Origin<'static>>,
    visible_participants: VisibleParticipants,
}

#[derive(Debug, Serialize)]
struct VisibleParticipants {
    participants: Vec<Participant>,
    redacted: bool,
}

fn to_event_view(event: Event, user: &User, next_event: Option<&NextEvent>) -> EventView {
    let is_next = next_event.is_some_and(|e| event.id == e.0.id);
    let visible_participants = visible_participants(&event, user, is_next);
    EventView {
        event,
        view_uri: is_next.then(|| uri!(play_page())),
        visible_participants,
    }
}

fn visible_participants(event: &Event, user: &User, is_next_event: bool) -> VisibleParticipants {
    if is_participating(event, user) || user.role == Role::Admin || is_next_event {
        VisibleParticipants {
            participants: event.participants.clone(),
            redacted: false,
        }
    } else {
        // Only show the organizer if the user hasn't participated.
        VisibleParticipants {
            participants: find_participant(event, event.created_by.id)
                .into_iter()
                .collect(),
            redacted: true,
        }
    }
}

fn find_participant(event: &Event, user_id: UserId) -> Option<Participant> {
    event
        .participants
        .iter()
        .find(|p| p.user.id == user_id)
        .cloned()
}

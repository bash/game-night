use crate::{
    database::Repository,
    event::{Event, VisibleParticipants},
    play::NextEvent,
    template::PageBuilder,
    uri,
    users::User,
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
    view_uri: Origin<'static>,
    visible_participants: VisibleParticipants,
}

fn to_event_view(event: Event, user: &User, next_event: Option<&NextEvent>) -> EventView {
    let is_next = next_event.is_some_and(|e| event.id == e.0.id);
    let visible_participants = VisibleParticipants::from_event(&event, user, is_next);
    EventView {
        view_uri: uri!(crate::event::event_page(id = event.id)),
        event,
        visible_participants,
    }
}

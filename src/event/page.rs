use super::{ActiveEvent, StatefulEvent};
use crate::database::Repository;
use crate::play::play_page;
use crate::poll::{no_open_poll_page, open_poll_page};
use crate::template::PageBuilder;
use crate::users::User;
use anyhow::{Error, Result};
use rocket::http::Status;
use rocket::response::{Debug, Redirect};
use rocket::{get, routes, uri, Responder, Route};
use rocket_dyn_templates::{context, Template};
use StatefulEvent::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![events_entry_page, event_page]
}

#[get("/")]
pub(crate) async fn events_entry_page(
    user: User,
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
) -> Result<EventsResponse, Debug<Error>> {
    let active_events = active_events(&mut *repository).await?;
    match active_events.len() {
        0 => Ok(EventsResponse::Template(no_open_poll_page(user, page))),
        1 => {
            let event_page_uri = uri!(event_page(id = active_events[0].event_id()));
            Ok(EventsResponse::Redirect(Redirect::to(event_page_uri)))
        }
        _ => todo!(),
    }
}

async fn active_events(repository: &mut dyn Repository) -> Result<Vec<ActiveEvent>> {
    Ok(repository
        .get_stateful_events()
        .await?
        .into_iter()
        .filter_map(|e| ActiveEvent::try_from(e).ok())
        .collect())
}

#[derive(Responder)]
pub(crate) enum EventsResponse {
    Template(Template),
    Redirect(Redirect),
}

#[get("/event/<id>")]
pub(crate) async fn event_page(
    user: User,
    id: i64, // TODO: uri!() macro has trouble with type alias
    mut repository: Box<dyn Repository>,
    page: PageBuilder<'_>,
) -> Result<Template, EventError> {
    let event = repository.get_stateful_event(id).await?;
    match event {
        Some(Polling(poll)) => Ok(open_poll_page(user, poll, page, repository).await?),
        Some(Finalizing(_)) => Ok(page.render("poll/pending-finalization", context! {})),
        Some(Planned(event)) => Ok(play_page(event, page, user, false)),
        Some(Archived(event)) => Ok(play_page(event, page, user, true)),
        Some(Failed(_)) | None => Err(EventError::Status(Status::NotFound)),
    }
}

#[derive(Responder)]
pub(crate) enum EventError {
    Error(Debug<Error>),
    Status(Status),
}

impl From<Error> for EventError {
    fn from(value: Error) -> Self {
        EventError::Error(Debug(value))
    }
}

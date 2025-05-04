use super::{ActiveEvent, EventViewModel, StatefulEvent};
use crate::event::EventsQuery;
use crate::play::{play_page, PlayPageStage};
use crate::poll::{no_open_poll_page, open_poll_page};
use crate::responder;
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::users::{User, UsersQuery};
use itertools::Itertools;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, routes, uri, Route};
use rocket_dyn_templates::{context, Template};
use StatefulEvent::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![events_entry_page, event_page]
}

#[get("/")]
pub(crate) async fn events_entry_page(
    user: User,
    page: PageBuilder<'_>,
    mut events: EventsQuery,
) -> HttpResult<EventsResponse> {
    let active_events = events.active(&user).await?;
    match active_events.len() {
        0 => Ok(no_open_poll_page(user, page).into()),
        1 => {
            let event_page_uri = uri!(event_page(id = active_events[0].event_id()));
            Ok(Redirect::to(event_page_uri).into())
        }
        _ => Ok(choose_event_page(user, page, active_events).into()),
    }
}

fn choose_event_page(
    user: User,
    page: PageBuilder<'_>,
    active_events: Vec<ActiveEvent>,
) -> Template {
    let events: Vec<_> = active_events
        .into_iter()
        .sorted_by_key(|e| e.date())
        .map(|e| EventViewModel::from_event(e, &user))
        .collect();
    let archive_uri = uri!(crate::play::archive_page());
    page.render("play/choose", context! { events, archive_uri })
}

responder! {
    pub(crate) enum EventsResponse {
        Template(Template),
        Redirect(Box<Redirect>),
    }
}

#[get("/event/<id>")]
pub(crate) async fn event_page(
    user: User,
    id: i64, // TODO: uri!() macro has trouble with type alias
    mut events: EventsQuery,
    users_query: UsersQuery,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    let event = events.with_id(id, &user).await?;
    match event {
        Some(Polling(poll)) => Ok(open_poll_page(user, poll, page, users_query).await?),
        Some(Pending(_) | Finalizing(_)) => {
            Ok(page.render("poll/pending-finalization", context! {}))
        }
        Some(Planned(event)) => Ok(play_page(event, page, user, PlayPageStage::Planned)),
        Some(Cancelled(event)) => Ok(play_page(event, page, user, PlayPageStage::Cancelled)),
        Some(Archived(event)) => Ok(play_page(event, page, user, PlayPageStage::Archived)),
        Some(Failed(_)) | None => Err(Status::NotFound.into()),
    }
}

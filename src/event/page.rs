use super::{ActiveEvent, EventListComponent, EventViewModel, EventsQuery, StatefulEvent};
use crate::auth::UriProvider;
use crate::play::{play_page, PlayPage, PlayPageStage};
use crate::poll::{open_poll_page, NoOpenPollPage, OpenPollPage};
use crate::responder;
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::template_v2::prelude::*;
use crate::users::{User, UsersQuery};
use itertools::Itertools;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, routes, uri, Route};
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
        0 => Ok(Templated(NoOpenPollPage::for_user(user, page.build())).into()),
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
) -> Templated<ChooseEventPage> {
    let events: Vec<_> = active_events
        .into_iter()
        .sorted_by_key(|e| e.date())
        .map(|e| EventViewModel::from_event(e, &user))
        .collect();
    Templated(ChooseEventPage {
        events,
        uri: UriProvider::for_user(user),
        ctx: page.build(),
    })
}

#[derive(Debug, Template)]
#[template(path = "event/choose.html")]
pub(crate) struct ChooseEventPage {
    events: Vec<EventViewModel>,
    uri: UriProvider,
    ctx: PageContext,
}

responder! {
    pub(crate) enum EventsResponse {
        NoOpenPoll(Box<Templated<NoOpenPollPage>>),
        Choose(Box<Templated<ChooseEventPage>>),
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
) -> HttpResult<EventDetailPageResponse> {
    let event = events.with_id(id, &user).await?;
    match event {
        Some(Polling(poll)) => Ok(open_poll_page(user, poll, page, users_query).await?.into()),
        Some(Pending(_) | Finalizing(_)) => {
            let page = PendingFinalizationPage {
                user,
                ctx: page.build(),
            };
            Ok(Templated(page).into())
        }
        Some(Planned(event)) => Ok(play_page(event, page, user, PlayPageStage::Planned)?.into()),
        Some(Cancelled(event)) => {
            Ok(play_page(event, page, user, PlayPageStage::Cancelled)?.into())
        }
        Some(Archived(event)) => Ok(play_page(event, page, user, PlayPageStage::Archived)?.into()),
        Some(Failed(_)) | None => Err(Status::NotFound.into()),
    }
}

#[derive(Template, Debug)]
#[template(path = "poll/pending-finalization.html")]
pub(crate) struct PendingFinalizationPage {
    user: User,
    ctx: PageContext,
}

responder! {
    pub(crate) enum EventDetailPageResponse {
        Play(Box<Templated<PlayPage>>),
        Poll(Box<Templated<OpenPollPage>>),
        PendingFinalization(Box<Templated<PendingFinalizationPage>>),
    }
}

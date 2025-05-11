use crate::auth::UriProvider;
use crate::database::Repository;
use crate::event::{
    Event, EventId, EventsQuery, Ics, LongEventTitleComponent, PostalAddressComponent,
    StatefulEvent, VisibleParticipants,
};
use crate::poll::EventEmailSender;
use crate::result::HttpResult;
use crate::template_v2::prelude::*;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::User;
use crate::users::UserNameComponent;
use rocket::http::uri::Origin;
use rocket::http::Status as HttpStatus;
use rocket::response::Redirect;
use rocket::{get, post, routes, Route};

mod archive;
pub(crate) use archive::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        play_redirect,
        join,
        event_ics,
        archive_page,
        crate::event::leave_page,
        crate::event::leave_,
    ]
}

// This is a bit of an ugly workaround to
// make the login show play as the active chapter.
#[get("/play")]
pub(crate) fn play_redirect(_user: User) -> Redirect {
    Redirect::to(uri!(crate::home::home_page()))
}

pub(crate) fn play_page(
    event: Event,
    page: PageContextBuilder<'_>,
    user: User,
    stage: PlayPageStage,
) -> HttpResult<Templated<PlayPage>> {
    let is_planned = matches!(stage, PlayPageStage::Planned);
    let join_uri = (!event.is_participant(&user) && is_planned).then(|| uri!(join(id = event.id)));
    let leave_uri = (event.is_participant(&user) && is_planned)
        .then(|| uri!(crate::event::leave_page(id = event.id)));
    let participants = VisibleParticipants::from_event(&event, &user, is_planned);
    let template = PlayPage {
        ics_uri: uri!(event_ics(id = event.id)),
        event,
        join_uri,
        leave_uri,
        stage,
        participants,
        uri: UriProvider::for_user(user.clone()),
        user,
        ctx: page.build(),
    };
    Ok(Templated(template))
}

#[derive(Template, Debug)]
#[template(path = "event/details.html")]
pub(crate) struct PlayPage {
    event: Event,
    user: User,
    stage: PlayPageStage,
    participants: VisibleParticipants,
    join_uri: Option<Origin<'static>>,
    leave_uri: Option<Origin<'static>>,
    ics_uri: Origin<'static>,
    uri: UriProvider,
    ctx: PageContext,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlayPageStage {
    Planned,
    Cancelled,
    Archived,
}

#[post("/event/<id>/join")]
async fn join(
    id: EventId,
    user: User,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
    mut sender: EventEmailSender,
) -> HttpResult<Redirect> {
    let Some(StatefulEvent::Planned(event)) = events.with_id(id, &user).await? else {
        return Err(HttpStatus::NotFound.into());
    };
    repository.add_participant(event.id, user.id).await?;
    sender.send(&event, &user).await?;
    Ok(Redirect::to(uri!(crate::event::event_page(id = event.id))))
}

#[get("/event/<id>/event.ics")]
pub(crate) async fn event_ics(
    id: EventId,
    user: User,
    mut events: EventsQuery,
    uri_builder: UriBuilder,
) -> HttpResult<Ics> {
    let Some(StatefulEvent::Planned(event) | StatefulEvent::Archived(event)) =
        events.with_id(id, &user).await?
    else {
        return Err(HttpStatus::NotFound.into());
    };
    Ok(Ics::from_event(&event, &uri_builder)?)
}

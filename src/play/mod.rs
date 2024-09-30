use crate::database::Repository;
use crate::event::{Event, EventId, EventsQuery, StatefulEvent, VisibleParticipants};
use crate::fmt::LongEventTitle;
use crate::poll::{EventEmailSender, Location};
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::User;
use anyhow::Result;
use ics::components::Property;
use ics::parameters::TzIDParam;
use ics::properties::{
    Description, DtEnd, DtStart, Location as LocationProp, Status, Summary, URL,
};
use ics::{escape_text, ICalendar};
use rocket::http::Status as HttpStatus;
use rocket::response::Redirect;
use rocket::{get, post, routes, Responder, Route};
use rocket_dyn_templates::{context, Template};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

mod archive;
pub(crate) use archive::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![play_redirect, join, event_ics, archive_page]
}

// This is a bit of an ugly workaround to
// make the login show play as the active chapter.
#[get("/play")]
pub(crate) fn play_redirect(_user: User) -> Redirect {
    Redirect::to(uri!(crate::home_page()))
}

pub(crate) fn play_page(
    event: Event,
    page: PageBuilder<'_>,
    user: User,
    is_archived: bool,
) -> Template {
    let join_uri =
        (!event.is_participant(&user) && !is_archived).then(|| uri!(join(id = event.id)));
    let archive_uri = uri!(archive_page());
    let participants = VisibleParticipants::from_event(&event, &user, !is_archived);
    page.render(
        "play",
        context! { ics_uri: uri!(event_ics(id = event.id)), event: event, join_uri, archive_uri, is_archived, participants },
    )
}

// TODO: make event-specific
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
async fn event_ics(
    id: EventId,
    user: User,
    mut events: EventsQuery,
    uri_builder: UriBuilder<'_>,
) -> HttpResult<Ics> {
    let Some(StatefulEvent::Planned(event) | StatefulEvent::Archived(event)) =
        events.with_id(id, &user).await?
    else {
        return Err(HttpStatus::NotFound.into());
    };
    let calendar = to_calendar(&event, &uri_builder)?;
    Ok(Ics(calendar.to_string()))
}

// TODO: move to sub-mod
pub(crate) fn to_calendar<'a>(
    event: &'a Event,
    uri_builder: &'a UriBuilder<'a>,
) -> Result<ICalendar<'a>> {
    let mut calendar = ICalendar::new("2.0", "game-night");
    calendar.add_event(to_ical_event(event, uri_builder)?);
    Ok(calendar)
}

fn to_ical_event<'a>(event: &'a Event, uri_builder: &'a UriBuilder<'a>) -> Result<ics::Event<'a>> {
    let starts_at = *event.starts_at;
    let mut ical_event = ics::Event::new(event_uid(event), format_as_floating(starts_at)?);
    ical_event.push(Summary::new(escape_text(format!(
        "{}",
        LongEventTitle(&event.title)
    ))));
    ical_event.push(Description::new(escape_text(&event.description)));
    ical_event.push(URL::new(
        uri!(uri_builder, crate::event::event_page(id = event.id)).to_string(),
    ));
    ical_event.push(Status::confirmed());
    ical_event.push(LocationProp::new(escape_text(format_location(
        &event.location,
    ))));
    ical_event.push(with_cet(DtStart::new(format_as_floating(starts_at)?)));
    ical_event.push(with_cet(DtEnd::new(format_as_floating(
        event.estimated_ends_at(),
    )?)));
    Ok(ical_event)
}

fn with_cet<'a>(property: impl Into<Property<'a>>) -> Property<'a> {
    let mut property = property.into();
    property.add(TzIDParam::new("CET"));
    property
}

fn event_uid(event: &Event) -> String {
    format!("{}@game-night.tau.garden", event.starts_at.unix_timestamp())
}

fn format_location(location: &Location) -> String {
    format!(
        "«{nameplate}» floor {floor}, {street} {number}, {plz} {city}",
        floor = &location.floor,
        nameplate = &location.nameplate,
        street = &location.street,
        number = &location.street_number,
        plz = &location.plz,
        city = &location.city,
    )
}

fn format_as_floating(value: OffsetDateTime) -> Result<String> {
    const FORMAT: &[FormatItem<'_>] =
        format_description!("[year][month][day]T[hour][minute][second]");
    Ok(value
        .to_timezone(timezones::db::CET)
        .format(&FORMAT)?
        .to_string())
}

#[derive(Responder)]
#[response(content_type = "text/calendar;charset=utf-8")]
struct Ics(String);

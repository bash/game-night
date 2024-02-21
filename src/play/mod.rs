use crate::database::Repository;
use crate::event::Event;
use crate::poll::Location;
use crate::template::PageBuilder;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::User;
use anyhow::{Error, Result};
use ics::components::Property;
use ics::parameters::TzIDParam;
use ics::properties::{
    Description, DtEnd, DtStart, Location as LocationProp, Status, Summary, URL,
};
use ics::{escape_text, ICalendar};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::response::{Debug, Redirect};
use rocket::{async_trait, get, routes, Request, Responder, Route};
use rocket_dyn_templates::{context, Template};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

pub(crate) fn routes() -> Vec<Route> {
    routes![play_page, play_redirect, event_ics]
}

// This is a bit of an ugly workaround to
// make the login show play as the active chapter.
#[get("/play")]
fn play_redirect(_user: User) -> Redirect {
    Redirect::to(uri!(play_page()))
}

#[get("/", rank = 0)]
fn play_page(event: NextEvent, page: PageBuilder<'_>, _user: User) -> Template {
    page.render(
        "play",
        context! { event: event.0, ics_uri: uri!(event_ics()) },
    )
}

struct NextEvent(Event);

#[async_trait]
impl<'r> FromRequest<'r> for NextEvent {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut repository: Box<dyn Repository> = try_outcome!(request.guard().await);
        match repository.get_next_event().await {
            Ok(Some(event)) => Outcome::Success(NextEvent(event)),
            Ok(None) => Outcome::Forward(rocket::http::Status::NotFound),
            Err(e) => Outcome::Error((rocket::http::Status::InternalServerError, e)),
        }
    }
}

#[get("/event.ics")]
async fn event_ics(
    event: NextEvent,
    uri_builder: UriBuilder<'_>,
    _user: User,
) -> Result<Ics, Debug<Error>> {
    let calendar = to_calendar(&event.0, &uri_builder)?;
    Ok(Ics(calendar.to_string()))
}

pub(crate) fn to_calendar<'a>(
    event: &'a Event,
    uri_builder: &'a UriBuilder<'a>,
) -> Result<ICalendar<'a>> {
    let mut calendar = ICalendar::new("2.0", "game-night");
    calendar.add_event(to_ical_event(event, uri_builder)?);
    Ok(calendar)
}

fn to_ical_event<'a>(event: &'a Event, uri_builder: &'a UriBuilder<'a>) -> Result<ics::Event<'a>> {
    let mut ical_event = ics::Event::new(event_uid(event), format_as_floating(event.starts_at)?);
    ical_event.push(Summary::new(escape_text("Tau's Game Night")));
    ical_event.push(Description::new(escape_text(&event.description)));
    ical_event.push(URL::new(uri!(uri_builder, play_page()).to_string()));
    ical_event.push(Status::confirmed());
    ical_event.push(LocationProp::new(escape_text(format_location(
        &event.location,
    ))));
    ical_event.push(with_cet(DtStart::new(format_as_floating(event.starts_at)?)));
    ical_event.push(with_cet(DtEnd::new(format_as_floating(event.ends_at)?)));
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

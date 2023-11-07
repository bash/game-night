use crate::database::Repository;
use crate::event::Event;
use crate::poll::Location;
use crate::template::PageBuilder;
use crate::users::User;
use crate::UrlPrefix;
use anyhow::{Error, Result};
use ics::components::Property;
use ics::parameters::TzIDParam;
use ics::properties::{
    Description, DtEnd, DtStart, Location as LocationProp, Status, Summary, URL,
};
use ics::{escape_text, ICalendar};
use rocket::http::uri::Absolute;
use rocket::response::Debug;
use rocket::{get, routes, uri, Responder, Route};
use rocket_dyn_templates::{context, Template};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

pub(crate) fn routes() -> Vec<Route> {
    routes![play_page, event_ics]
}

#[get("/play")]
async fn play_page(
    mut repository: Box<dyn Repository>,
    page: PageBuilder<'_>,
    _user: User,
) -> Result<Template, Debug<Error>> {
    let event = repository.get_next_event().await?;
    Ok(page.render("play", context! { event }))
}

#[get("/play/event.ics")]
async fn event_ics(
    mut repository: Box<dyn Repository>,
    url_prefix: UrlPrefix<'_>,
    _user: User,
) -> Result<Ics, Debug<Error>> {
    let event = repository.get_next_event().await?;
    let calendar = to_calendar(event.as_ref(), &url_prefix.0)?;
    Ok(Ics(calendar.to_string()))
}

pub(crate) fn to_calendar<'a>(
    event: Option<&'a Event>,
    url_prefix: &'a Absolute<'a>,
) -> Result<ICalendar<'a>> {
    let mut calendar = ICalendar::new("2.0", "game-night");

    if let Some(event) = event {
        calendar.add_event(to_ical_event(event, url_prefix)?);
    }

    Ok(calendar)
}

fn to_ical_event<'a>(event: &'a Event, url_prefix: &'a Absolute<'a>) -> Result<ics::Event<'a>> {
    let mut ical_event = ics::Event::new(event_uid(event), format_as_floating(event.starts_at)?);
    ical_event.push(Summary::new(escape_text("Tau's Game Night")));
    ical_event.push(Description::new(escape_text(&event.description)));
    ical_event.push(URL::new(uri!(url_prefix.clone(), play_page()).to_string()));
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

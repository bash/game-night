use crate::event::Event;
use crate::fmt::LongEventTitle;
use crate::poll::Location;
use crate::uri;
use crate::uri::UriBuilder;
use anyhow::Result;
use ics::components::Property;
use ics::parameters::TzIDParam;
use ics::properties::{
    Description, DtEnd, DtStart, Location as LocationProp, Status, Summary, URL,
};
use ics::{escape_text, ICalendar};
use rocket::Responder;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use time_tz::{timezones, OffsetDateTimeExt};

#[derive(Responder)]
#[response(content_type = "text/calendar;charset=utf-8")]
pub(crate) struct Ics(pub(crate) String);

impl Ics {
    pub(crate) fn from_event(event: &Event, uri_builder: &UriBuilder<'_>) -> Result<Ics> {
        let mut calendar = ICalendar::new("2.0", "game-night");
        calendar.add_event(to_ical_event(event, uri_builder)?);
        Ok(Ics(calendar.to_string()))
    }
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

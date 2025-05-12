use crate::iso_8601::Iso8601;
use crate::template::prelude::*;
use itertools::Itertools as _;
use std::iter;
use time::format_description::BorrowedFormatItem as FormatItem;
use time::macros::format_description;
use time::{Date, Month, OffsetDateTime, Time};

#[derive(Template, Debug)]
#[template(path = "poll/calendar.html")]
pub(crate) struct Calendar {
    months: Vec<CalendarMonth>,
}

impl Calendar {
    pub(crate) fn generate(
        start: OffsetDateTime,
        days: usize,
        prefill: &mut impl Fn(Date) -> CalendarDayPrefill,
    ) -> Self {
        let months = iter::successors(Some(start.date()), |d| d.next_day())
            .take(days)
            .chunk_by(|d| d.month())
            .into_iter()
            .map(|(month, days)| to_calendar_month(month, days, prefill))
            .collect();
        Self { months }
    }
}

#[derive(Debug, rocket::FromForm, Clone)]
pub(crate) struct CalendarDayPrefill {
    pub(crate) date: Iso8601<Date>,
    pub(crate) enabled: bool,
    pub(crate) start_time: Option<Iso8601<Time>>,
}

impl CalendarDayPrefill {
    pub(crate) fn empty(date: impl Into<Iso8601<Date>>) -> Self {
        Self {
            date: date.into(),
            enabled: false,
            start_time: None,
        }
    }
}

fn to_calendar_month(
    month: Month,
    days: impl Iterator<Item = Date>,
    prefill: &mut impl Fn(Date) -> CalendarDayPrefill,
) -> CalendarMonth {
    CalendarMonth {
        name: month.to_string(),
        days: days.map(|d| to_calendar_day(d, prefill)).collect(),
    }
}

fn to_calendar_day(date: Date, prefill: &mut impl Fn(Date) -> CalendarDayPrefill) -> CalendarDay {
    const WEEKDAY_FORMAT: &[FormatItem<'_>] = format_description!("[weekday repr:long]");
    CalendarDay {
        date: date.into(),
        day: date.day(),
        weekday: date.format(WEEKDAY_FORMAT).unwrap(),
        prefill: prefill(date),
    }
}

#[derive(Debug)]
struct CalendarMonth {
    name: String,
    days: Vec<CalendarDay>,
}

#[derive(Debug)]
struct CalendarDay {
    date: Iso8601<Date>,
    day: u8,
    weekday: String,
    prefill: CalendarDayPrefill,
}

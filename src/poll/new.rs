use crate::authorization::{AuthorizedTo, ManagePoll};
use crate::template::{PageBuilder, PageType};
use anyhow::Error;
use itertools::Itertools;
use rocket::response::Debug;
use rocket::{get, post};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::iter;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Date, Month, OffsetDateTime};

#[post("/poll/new")]
pub(super) fn new_poll(_user: AuthorizedTo<ManagePoll>) {}

#[get("/poll/new")]
pub(super) fn new_poll_page(
    page: PageBuilder<'_>,
    _user: AuthorizedTo<ManagePoll>,
) -> Result<Template, Debug<Error>> {
    let calendar = get_calendar(OffsetDateTime::now_utc(), 14);
    Ok(page
        .type_(PageType::Poll)
        .render("poll/new", context! { calendar }))
}

fn get_calendar(start: OffsetDateTime, days: usize) -> Vec<CalendarMonth> {
    iter::successors(Some(start.date()), |d| d.next_day())
        .take(days)
        .group_by(|d| d.month())
        .into_iter()
        .map(|(month, days)| to_calendar_month(month, days))
        .collect()
}

fn to_calendar_month(month: Month, days: impl Iterator<Item = Date>) -> CalendarMonth {
    CalendarMonth {
        name: month.to_string(),
        days: days.map(to_calendar_day).collect(),
    }
}

fn to_calendar_day(date: Date) -> CalendarDay {
    const WEEKDAY_FORMAT: &[FormatItem<'_>] = format_description!("[weekday repr:long]");
    CalendarDay {
        date,
        day: date.day(),
        weekday: date.format(WEEKDAY_FORMAT).unwrap(),
    }
}

#[derive(Debug, Serialize)]
struct CalendarMonth {
    name: String,
    days: Vec<CalendarDay>,
}

#[derive(Debug, Serialize)]
struct CalendarDay {
    date: Date,
    day: u8,
    weekday: String,
}

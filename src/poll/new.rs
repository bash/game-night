use crate::authorization::{AuthorizedTo, ManagePoll};
use crate::template::{PageBuilder, PageType};
use chrono::{Datelike, Local, Month, NaiveDate};
use itertools::Itertools;
use rocket::{get, post};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[post("/poll/new")]
pub(super) fn new_poll(_user: AuthorizedTo<ManagePoll>) {}

#[get("/poll/new")]
pub(super) fn new_poll_page(page: PageBuilder<'_>, _user: AuthorizedTo<ManagePoll>) -> Template {
    let calendar = get_calendar(Local::now().date_naive(), 14);
    page.type_(PageType::Poll)
        .render("poll/new", context! { calendar })
}

fn get_calendar(start: NaiveDate, days: usize) -> Vec<CalendarMonth> {
    start
        .iter_days()
        .take(days)
        .group_by(get_month)
        .into_iter()
        .map(|(month, days)| to_calendar_month(month, days))
        .collect()
}

fn get_month(date: &impl Datelike) -> Month {
    Month::try_from(u8::try_from(date.month()).unwrap()).unwrap()
}

fn to_calendar_month(month: Month, days: impl Iterator<Item = NaiveDate>) -> CalendarMonth {
    CalendarMonth {
        name: month.name(),
        days: days.map(to_calendar_day).collect(),
    }
}

fn to_calendar_day(date: NaiveDate) -> CalendarDay {
    CalendarDay {
        date,
        day: date.day(),
        weekday: date.format("%a").to_string(),
    }
}

#[derive(Debug, Serialize)]
struct CalendarMonth {
    name: &'static str,
    days: Vec<CalendarDay>,
}

#[derive(Debug, Serialize)]
struct CalendarDay {
    date: NaiveDate,
    day: u32,
    weekday: String,
}

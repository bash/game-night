use super::rocket_uri_macro_poll_page;
use super::{DateSelectionStrategy, Poll, PollOption};
use crate::authorization::{AuthorizedTo, ManagePoll};
use crate::database::Repository;
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use anyhow::{Context, Error, Result};
use itertools::Itertools;
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri, FromForm};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::iter;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time};
use time_tz::{timezones, PrimitiveDateTimeExt};

#[get("/poll/new")]
pub(super) fn new_poll_page(
    page: PageBuilder<'_>,
    _user: AuthorizedTo<ManagePoll>,
) -> Result<Template, Debug<Error>> {
    let calendar = get_calendar(OffsetDateTime::now_utc(), 90);
    Ok(page
        .type_(PageType::Poll)
        .render("poll/new", context! { calendar, strategies: strategies() }))
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
struct DateSelectionStrategyOption {
    name: &'static str,
    value: DateSelectionStrategy,
}

fn strategies() -> &'static [DateSelectionStrategyOption] {
    &[
        DateSelectionStrategyOption {
            name: "at random",
            value: DateSelectionStrategy::AtRandom,
        },
        DateSelectionStrategyOption {
            name: "to maximize participants",
            value: DateSelectionStrategy::ToMaximizeParticipants,
        },
    ]
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

#[post("/poll/new", data = "<form>")]
pub(super) async fn new_poll(
    mut repository: Box<dyn Repository>,
    form: Form<NewPollData<'_>>,
    user: AuthorizedTo<ManagePoll>,
) -> Result<Redirect, Debug<Error>> {
    let poll = to_poll(form.into_inner(), &user)?;
    repository.add_poll(poll).await?;
    Ok(Redirect::to(uri!(poll_page())))
}

fn to_poll(poll: NewPollData, user: &User) -> Result<Poll<(), UserId>> {
    let now = now_utc_without_subminutes()?;
    Ok(Poll {
        id: (),
        min_participants: poll.min_participants,
        max_participants: poll.max_participants,
        strategy: poll.strategy,
        description: poll.description.to_owned(),
        open_until: now + Duration::hours(poll.duration_in_hours),
        closed: false,
        created_by: user.id,
        options: to_poll_options(poll.options.iter())?,
    })
}

fn to_poll_options<'a>(
    options: impl Iterator<Item = &'a NewPollOption>,
) -> Result<Vec<PollOption<(), UserId>>> {
    options.filter(|o| o.enabled).map(to_poll_option).collect()
}

fn to_poll_option(option: &NewPollOption) -> Result<PollOption<(), UserId>> {
    Ok(PollOption {
        id: (),
        datetime: to_cet(option.date, option.time)?,
        answers: Vec::default(),
    })
}

fn to_cet(date: Date, time: Time) -> Result<OffsetDateTime> {
    PrimitiveDateTime::new(date, time)
        .assume_timezone(timezones::db::CET)
        .take()
        .context("Offset is ambigious")
}

fn now_utc_without_subminutes() -> Result<OffsetDateTime> {
    Ok(OffsetDateTime::now_utc()
        .replace_second(0)?
        .replace_nanosecond(0)?)
}

#[derive(Debug, FromForm, Serialize)]
pub(super) struct NewPollData<'r> {
    min_participants: u64,
    #[field(validate = gte(self.min_participants))]
    max_participants: u64,
    strategy: DateSelectionStrategy,
    #[field(name = "duration", validate = gte(1))]
    duration_in_hours: i64,
    description: &'r str,
    options: Vec<NewPollOption>,
}

#[derive(Debug, Serialize, FromForm)]
pub(super) struct NewPollOption {
    date: Date,
    #[field(default_with = Some(Time::MIDNIGHT))]
    time: Time,
    enabled: bool,
}

pub fn gte<'v, A, B>(a: &A, b: B) -> rocket::form::Result<'v, ()>
where
    A: PartialOrd<B>,
{
    if a >= &b {
        Ok(())
    } else {
        Err(rocket::form::Error::validation("value does not match expected value").into())
    }
}

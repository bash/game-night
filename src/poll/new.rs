use super::PollEmail;
use super::{rocket_uri_macro_poll_page, Answer, AnswerValue, Attendance, Location};
use super::{DateSelectionStrategy, Poll, PollOption};
use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::Repository;
use crate::email::EmailSender;
use crate::login::{with_autologin_token, LoginToken};
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use crate::UrlPrefix;
use anyhow::{Context as _, Error, Result};
use itertools::Itertools;
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri, FromForm, State};
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
    let calendar = get_calendar(OffsetDateTime::now_utc(), 14);
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
    name: String,
    value: DateSelectionStrategy,
}

impl From<DateSelectionStrategy> for DateSelectionStrategyOption {
    fn from(value: DateSelectionStrategy) -> Self {
        DateSelectionStrategyOption {
            name: value.to_string(),
            value,
        }
    }
}

fn strategies() -> [DateSelectionStrategyOption; 2] {
    [
        DateSelectionStrategy::AtRandom.into(),
        DateSelectionStrategy::ToMaximizeParticipants.into(),
    ]
}

#[derive(Debug, Serialize)]
struct CalendarMonth {
    name: String,
    days: Vec<CalendarDay>,
}

#[derive(Debug, Serialize)]
struct CalendarDay {
    #[serde(with = "iso8601_date")]
    date: Date,
    day: u8,
    weekday: String,
}

time::serde::format_description!(iso8601_date, Date, "[year]-[month]-[day]");

#[post("/poll/new", data = "<form>")]
pub(super) async fn new_poll(
    mut repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    form: Form<NewPollData<'_>>,
    user: AuthorizedTo<ManagePoll>,
    url_prefix: UrlPrefix<'_>,
) -> Result<Redirect, Debug<Error>> {
    let location = repository.get_location().await?;
    let poll = to_poll(form.into_inner(), location, &user)?;
    repository.add_poll(&poll).await?;
    send_poll_emails(repository, email_sender.as_ref(), url_prefix, &poll).await?;
    Ok(Redirect::to(uri!(poll_page())))
}

fn to_poll(poll: NewPollData, location: Location, user: &User) -> Result<Poll<(), UserId, i64>> {
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
        location: location.id,
        options: to_poll_options(poll.options.iter(), user)?,
    })
}

fn to_poll_options<'a>(
    options: impl Iterator<Item = &'a NewPollOption>,
    user: &User,
) -> Result<Vec<PollOption<(), UserId>>> {
    options
        .filter(|o| o.enabled)
        .map(|o| to_poll_option(o, user))
        .collect()
}

fn to_poll_option(option: &NewPollOption, user: &User) -> Result<PollOption<(), UserId>> {
    Ok(PollOption {
        id: (),
        datetime: to_cet(option.date, option.time)?,
        // The user creating the poll is automatically added with a required attendance.
        answers: vec![Answer {
            id: (),
            user: user.id,
            value: AnswerValue::yes(Attendance::Required),
        }],
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

#[derive(Debug, FromForm)]
pub(super) struct NewPollData<'r> {
    min_participants: usize,
    #[field(validate = gte(self.min_participants))]
    max_participants: usize,
    strategy: DateSelectionStrategy,
    #[field(name = "duration", validate = gte(1))]
    duration_in_hours: i64,
    description: &'r str,
    options: Vec<NewPollOption>,
}

#[derive(Debug, FromForm)]
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

async fn send_poll_emails(
    mut repository: Box<dyn Repository>,
    email_sender: &dyn EmailSender,
    url_prefix: UrlPrefix<'_>,
    poll: &Poll<(), UserId, i64>,
) -> Result<()> {
    for user in repository.get_users().await? {
        let token = LoginToken::generate_reusable(user.id, poll.open_until);
        repository.add_login_token(&token).await?;
        let poll_url = with_autologin_token(uri!(url_prefix.0.clone(), poll_page()), &token);
        let email = PollEmail {
            name: user.name.clone(),
            poll_closes_at: poll.open_until,
            poll_url,
        };
        email_sender.send(user.mailbox()?, &email).await?;
    }

    Ok(())
}

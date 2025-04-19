use super::{Answer, AnswerValue, Attendance};
use super::{DateSelectionStrategy, Poll, PollOption};
use super::{PollEmail, PollStage};
use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::{New, Repository};
use crate::event::{
    rocket_uri_macro_event_page, Event, EventEmailSender, EventsQuery, Location, Polling,
    StatefulEvent,
};
use crate::iso_8601::Iso8601;
use crate::register::rocket_uri_macro_profile;
use crate::template::PageBuilder;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::{SubscribedUsers, User};
use anyhow::{Context as _, Error, Result};
use itertools::Itertools as _;
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, FromForm};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::iter;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time};
use time_tz::{timezones, PrimitiveDateTimeExt};

#[get("/poll/new")]
pub(super) async fn new_poll_page(
    user: AuthorizedTo<ManagePoll>,
    page: PageBuilder<'_>,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
) -> Result<Template, Debug<Error>> {
    let calendar = get_calendar(
        OffsetDateTime::now_utc(),
        14,
        &mut CalendarDayPrefill::empty,
    );
    let description = events.newest(&user).await?.map(|e| e.description);
    let groups = repository.get_groups().await?;
    let locations = repository.get_locations().await?;
    Ok(page.render(
        "poll/new",
        context! { calendar, strategies: strategies(), calendar_uri: uri!(calendar()), description, groups, locations },
    ))
}

#[post("/poll/new/_calendar", data = "<form>")]
pub(super) fn calendar(
    page: PageBuilder<'_>,
    _user: AuthorizedTo<ManagePoll>,
    form: Form<CalendarData>,
) -> Template {
    let mut prefill = find_prefill(&form);
    let calendar = get_calendar(OffsetDateTime::now_utc(), 14 * form.count, &mut prefill);
    page.render(
        "poll/calendar",
        context! { calendar, strategies: strategies() },
    )
}

fn find_prefill(form: &CalendarData) -> impl Fn(Date) -> CalendarDayPrefill + '_ {
    |date| {
        form.options
            .iter()
            .find(|o| *o.date == date)
            .cloned()
            .unwrap_or_else(|| CalendarDayPrefill::empty(date))
    }
}

#[derive(FromForm)]
pub(super) struct CalendarData {
    count: usize,
    options: Vec<CalendarDayPrefill>,
}

fn get_calendar(
    start: OffsetDateTime,
    days: usize,
    prefill: &mut impl Fn(Date) -> CalendarDayPrefill,
) -> Vec<CalendarMonth> {
    iter::successors(Some(start.date()), |d| d.next_day())
        .take(days)
        .chunk_by(|d| d.month())
        .into_iter()
        .map(|(month, days)| to_calendar_month(month, days, prefill))
        .collect()
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
    date: Iso8601<Date>,
    day: u8,
    weekday: String,
    prefill: CalendarDayPrefill,
}

#[derive(Debug, Serialize, FromForm, Clone)]
struct CalendarDayPrefill {
    date: Iso8601<Date>,
    enabled: bool,
    start_time: Option<Iso8601<Time>>,
}

impl CalendarDayPrefill {
    fn empty(date: impl Into<Iso8601<Date>>) -> Self {
        Self {
            date: date.into(),
            enabled: false,
            start_time: None,
        }
    }
}

#[post("/poll/new", data = "<form>")]
pub(super) async fn new_poll(
    mut repository: Box<dyn Repository>,
    subscribed_users: SubscribedUsers,
    email_sender: Box<dyn EventEmailSender<Polling>>,
    form: Form<NewPollData<'_>>,
    user: AuthorizedTo<ManagePoll>,
    uri_builder: UriBuilder,
) -> Result<Redirect, Debug<Error>> {
    let location = repository
        .get_location_by_id(form.location)
        .await?
        .context("invalid location id")?;
    let new_poll = to_poll(form.into_inner(), location, &user)?;
    let poll = repository.add_poll(new_poll).await?;
    send_poll_emails(subscribed_users, email_sender, uri_builder, &poll).await?;
    Ok(Redirect::to(uri!(event_page(id = poll.event.id))))
}

fn to_poll(poll: NewPollData, location: Location, user: &User) -> Result<Poll<New>> {
    let now = now_utc_without_subminutes()?;
    Ok(Poll {
        id: (),
        min_participants: poll.min_participants,
        strategy: poll.strategy,
        open_until: (now + Duration::hours(poll.duration_in_hours)).into(),
        stage: PollStage::Open,
        options: to_poll_options(poll.options.iter(), user)?,
        event: Event {
            id: (),
            title: poll.title.to_owned(),
            description: poll.description.to_owned(),
            created_by: user.id,
            location: location.id,
            participants: vec![],
            starts_at: None,
            restrict_to: poll.restrict_to,
            cancelled: false,
            parent_id: None,
        },
    })
}

fn to_poll_options<'a>(
    options: impl Iterator<Item = &'a NewPollOption>,
    user: &User,
) -> Result<Vec<PollOption<New>>> {
    options
        .filter(|o| o.enabled)
        .map(|o| to_poll_option(o, user))
        .collect()
}

fn to_poll_option(option: &NewPollOption, user: &User) -> Result<PollOption<New>> {
    Ok(PollOption {
        id: (),
        starts_at: to_cet(option.date, option.start_time)?.into(),
        promote: false,
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
        .replace_minute(0)?
        .replace_second(0)?
        .replace_nanosecond(0)?)
}

#[derive(Debug, FromForm)]
pub(super) struct NewPollData<'r> {
    #[field(validate = gte(2))]
    min_participants: usize,
    strategy: DateSelectionStrategy,
    #[field(name = "duration", validate = gte(1))]
    duration_in_hours: i64,
    title: &'r str,
    description: &'r str,
    options: Vec<NewPollOption>,
    restrict_to: Option<i64>,
    location: i64,
}

#[derive(Debug, FromForm)]
pub(super) struct NewPollOption {
    date: Date,
    #[field(default_with = Some(Time::MIDNIGHT))]
    start_time: Time,
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
    mut subscribed_users: SubscribedUsers,
    mut email_sender: Box<dyn EventEmailSender<Polling>>,
    uri_builder: UriBuilder,
    poll: &Poll,
) -> Result<()> {
    let event = StatefulEvent::from_poll(poll.clone(), OffsetDateTime::now_utc());
    for user in subscribed_users.for_event(&event).await? {
        let open_until = *poll.open_until;
        let poll_uri = uri!(auto_login(&user, open_until); uri_builder, crate::event::event_page(id = poll.event.id)).await?;
        let skip_poll_uri =
            uri!(auto_login(&user, open_until); uri_builder, super::skip::skip_poll(id = poll.event.id)).await?;
        let sub_url = uri!(auto_login(&user, open_until); uri_builder, profile()).await?;
        let email = PollEmail {
            name: user.name.clone(),
            poll: poll.clone(),
            poll_uri,
            skip_poll_uri,
            manage_subscription_url: sub_url,
        };
        email_sender.send(&poll.event, &user, &email).await?;
    }

    Ok(())
}

use super::calendar::{Calendar, CalendarDayPrefill};
use super::{Answer, AnswerValue, Attendance};
use super::{DateSelectionStrategy, Poll, PollOption};
use super::{PollEmail, PollStage};
use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::{New, Repository};
use crate::event::{
    rocket_uri_macro_event_page, Event, EventEmailSender, EventsQuery, Location, Polling,
    StatefulEvent,
};
use crate::groups::Group;
use crate::push::{PollNotification, PushSender};
use crate::register::rocket_uri_macro_profile;
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::uri::UriBuilder;
use crate::users::{SubscribedUsers, User};
use crate::{auto_resolve, uri};
use anyhow::{Context as _, Result};
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::response::Redirect;
use rocket::{get, post, FromForm};
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, Time};
use time_tz::{timezones, PrimitiveDateTimeExt};

#[get("/poll/new")]
pub(crate) async fn new_poll_page(
    user: AuthorizedTo<ManagePoll>,
    page: PageContextBuilder<'_>,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Templated<NewPollPage>> {
    let calendar = Calendar::generate(
        OffsetDateTime::now_utc(),
        14,
        &mut CalendarDayPrefill::empty,
    );
    let description = events.newest(&user).await?.map(|e| e.description);
    let groups = repository.get_groups().await?;
    let locations = repository.get_locations().await?;
    let page = NewPollPage {
        calendar,
        strategies: strategies().into_iter().collect(),
        calendar_uri: uri!(calendar()),
        description,
        groups,
        locations,
        ctx: page.build(),
    };
    Ok(Templated(page))
}

#[derive(Template, Debug)]
#[template(path = "poll/new.html")]
pub(crate) struct NewPollPage {
    calendar: Calendar,
    description: Option<String>,
    groups: Vec<Group>,
    calendar_uri: Origin<'static>,
    locations: Vec<Location>,
    strategies: Vec<DateSelectionStrategyOption>,
    ctx: PageContext,
}

#[post("/poll/new/_calendar", data = "<form>")]
pub(super) fn calendar(
    _user: AuthorizedTo<ManagePoll>,
    form: Form<CalendarData>,
) -> Templated<Calendar> {
    let mut prefill = find_prefill(&form);
    Templated(Calendar::generate(
        OffsetDateTime::now_utc(),
        14 * form.count,
        &mut prefill,
    ))
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

#[derive(Debug)]
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

#[post("/poll/new", data = "<form>")]
pub(super) async fn new_poll(
    mut repository: Box<dyn Repository>,
    form: Form<NewPollData<'_>>,
    user: AuthorizedTo<ManagePoll>,
    mut notification_sender: NewPollNotificationSender,
) -> HttpResult<Redirect> {
    let location = repository
        .get_location_by_id(form.location)
        .await?
        .context("invalid location id")?;
    let new_poll = to_poll(form.into_inner(), location, &user)?;
    let poll = repository.add_poll(new_poll).await?;
    notification_sender.execute(&poll).await?;
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

auto_resolve! {
    pub(crate) struct NewPollNotificationSender {
        subscribed_users: SubscribedUsers,
        email_sender: Box<dyn EventEmailSender<Polling>>,
        push_sender: PushSender,
        uri_builder: UriBuilder,
    }
}

impl NewPollNotificationSender {
    async fn execute(&mut self, poll: &Poll) -> Result<()> {
        let event = StatefulEvent::from_poll(poll.clone(), OffsetDateTime::now_utc());
        for user in self.subscribed_users.for_event(&event).await? {
            let open_until = *poll.open_until;
            let poll_uri = uri!(auto_login(&user, open_until); self.uri_builder, crate::event::event_page(id = poll.event.id)).await?;
            let skip_poll_uri =
                uri!(auto_login(&user, open_until); self.uri_builder, super::skip::skip_poll(id = poll.event.id)).await?;
            let sub_url = uri!(auto_login(&user, open_until); self.uri_builder, profile()).await?;
            let email = PollEmail {
                name: user.name.clone(),
                poll: poll.clone(),
                poll_uri,
                skip_poll_uri,
                manage_subscription_url: sub_url,
            };
            self.email_sender.send(&poll.event, &user, &email).await?;
            let notification = PollNotification { poll };
            self.push_sender
                .send_templated(&notification, user.id)
                .await?;
        }

        Ok(())
    }
}

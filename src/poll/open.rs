use super::ParticipatedMessageComponent;
use super::{Answer, AnswerValue};
use crate::auth::can_answer_strongly;
use crate::database::{New, Repository};
use crate::event::LongEventTitleComponent;
use crate::event::{EventsQuery, StatefulEvent};
use crate::iso_8601::Iso8601;
use crate::poll::{Poll, PollOption};
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::users::{User, UserNameComponent, UserQueries};
use itertools::{Either, Itertools as _};
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{post, uri, FromForm};
use std::collections::HashMap;
use time::{Month, OffsetDateTime};

pub(crate) async fn open_poll_page(
    user: User,
    poll: Poll,
    page: PageContextBuilder<'_>,
    mut users: UserQueries,
) -> HttpResult<Templated<OpenPollPage>> {
    let event = StatefulEvent::from_poll(poll.clone(), OffsetDateTime::now_utc());
    let users = users.active_and_invited(&event).await?;
    let template = to_open_poll_page(poll, user, users, page.build());
    Ok(Templated(template))
}

#[derive(Template, Debug)]
#[template(path = "poll/open.html")]
pub(crate) struct OpenPollPage {
    poll: Poll,
    option_groups: Vec<OpenPollOptionsGroup>,
    has_answers: bool,
    can_answer_strongly: bool,
    no_date_answered_with_yes: Vec<User>,
    not_answered: Vec<User>,
    update_answers_uri: Origin<'static>,
    close_poll_uri: Option<Origin<'static>>,
    user: User,
    ctx: PageContext,
}

fn to_open_poll_page(poll: Poll, user: User, users: Vec<User>, ctx: PageContext) -> OpenPollPage {
    let (not_answered, no_date_answered_with_yes) = if user.can_manage_poll() {
        users_with_no_yes(&poll, users)
    } else {
        Default::default()
    };

    OpenPollPage {
        option_groups: to_open_poll_options(&poll, poll.options.iter(), &user),
        has_answers: poll.has_answer(user.id),
        can_answer_strongly: can_answer_strongly(&user, &poll),
        update_answers_uri: uri!(update_answers(id = poll.event.id)),
        close_poll_uri: user
            .can_manage_poll()
            .then(|| uri!(super::admin::close_poll_page(id = poll.event.id))),
        poll,
        not_answered,
        no_date_answered_with_yes,
        user,
        ctx,
    }
}

fn users_with_no_yes(poll: &Poll, users: Vec<User>) -> (Vec<User>, Vec<User>) {
    let (answered, not_answered) = partition_by_answered(poll, users);
    let no_date_answered_with_yes = answered
        .into_iter()
        .filter(|u| !poll.has_yes_answer(u.id))
        .collect();
    (not_answered, no_date_answered_with_yes)
}

fn partition_by_answered(poll: &Poll, users: Vec<User>) -> (Vec<User>, Vec<User>) {
    users.into_iter().partition_map(|user| {
        if poll.has_answer(user.id) {
            Either::Left(user)
        } else {
            Either::Right(user)
        }
    })
}

fn to_open_poll_options<'a>(
    poll: &Poll,
    options: impl Iterator<Item = &'a PollOption>,
    user: &User,
) -> Vec<OpenPollOptionsGroup> {
    options
        .filter(|o: &&PollOption| !o.has_veto() || can_answer_strongly(user, poll))
        .chunk_by(|o| o.starts_at.month())
        .into_iter()
        .map(|(month, options)| to_open_poll_options_group(month, options, user))
        .collect()
}

fn to_open_poll_options_group<'a>(
    month: Month,
    options: impl Iterator<Item = &'a PollOption>,
    user: &User,
) -> OpenPollOptionsGroup {
    let options = options.map(|o| to_open_poll_option(o, user)).collect();
    OpenPollOptionsGroup {
        name: month.to_string(),
        options,
    }
}

fn to_open_poll_option(option: &PollOption, user: &User) -> OpenPollOption {
    let answer = option
        .answers
        .iter()
        .find(|a| a.user.id == user.id)
        .map(|a| a.value);
    let (yes, strong) = answer.map(|a| a.to_bools()).unwrap_or_default();
    OpenPollOption {
        id: option.id,
        starts_at: option.starts_at,
        yes,
        strong,
        vetoed: option.has_veto(),
        yes_answers: option
            .answers
            .iter()
            .filter(|a| a.value.is_yes() && a.user.id != user.id)
            .map(|a| a.user.clone())
            .collect(),
    }
}

#[derive(Debug)]
struct OpenPollOptionsGroup {
    name: String,
    options: Vec<OpenPollOption>,
}

#[derive(Debug)]
struct OpenPollOption {
    id: i64,
    starts_at: Iso8601<OffsetDateTime>,
    yes: bool,
    strong: bool,
    vetoed: bool,
    yes_answers: Vec<User>,
}

#[post("/event/<id>/poll/answers", data = "<form>")]
pub(super) async fn update_answers(
    id: i64,
    user: User,
    form: Form<AnswerUpdates>,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::BadRequest.into());
    };
    repository
        .add_answers(apply_updates(&poll, &user, form.into_inner()))
        .await?;
    Ok(Redirect::to(uri!(crate::event::event_page(
        id = poll.event.id
    ))))
}

fn apply_updates(poll: &Poll, user: &User, updates: AnswerUpdates) -> Vec<(i64, Answer<New>)> {
    poll.options
        .iter()
        .map(|option| (option.id, get_answer(user, &updates, option)))
        .collect()
}

fn get_answer(user: &User, updates: &AnswerUpdates, option: &PollOption) -> Answer<New> {
    Answer {
        id: (),
        user: user.id,
        value: get_answer_value(updates, option.id, user),
    }
}

fn get_answer_value(updates: &AnswerUpdates, option_id: i64, user: &User) -> AnswerValue {
    let update = updates.options.get(&option_id).copied().unwrap_or_default();
    let value = AnswerValue::from_bools((update.yes, update.strong));
    if user.can_answer_strongly() {
        value
    } else {
        value.ensure_weak()
    }
}

#[derive(Debug, FromForm)]
pub(super) struct AnswerUpdates {
    options: HashMap<i64, AnswerUpdate>,
}

#[derive(Debug, Copy, Clone, Default, FromForm)]
pub(super) struct AnswerUpdate {
    yes: bool,
    strong: bool,
}

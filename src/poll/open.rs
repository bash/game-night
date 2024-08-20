use super::Open;
use super::{Answer, AnswerValue};
use crate::database::Repository;
use crate::poll::{Poll, PollOption};
use crate::template::PageBuilder;
use crate::users::{User, UserId};
use anyhow::Error;
use itertools::{Either, Itertools as _};
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri, FromForm};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::collections::HashMap;
use time::{Month, OffsetDateTime};

#[get("/", rank = 10)]
pub(crate) async fn open_poll_page(
    user: User,
    poll: Open<Poll>,
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
) -> Result<Template, Debug<Error>> {
    let users = repository.get_users().await?;
    Ok(page.render("poll/open", to_open_poll(poll.into_inner(), &user, users)))
}

fn to_open_poll(poll: Poll, user: &User, users: Vec<User>) -> OpenPoll {
    let (not_answered, no_date_answered_with_yes) = if user.can_manage_poll() {
        users_with_no_yes(&poll, users)
    } else {
        Default::default()
    };

    OpenPoll {
        option_groups: to_open_poll_options(poll.options.iter(), user),
        date_selection_strategy: poll.strategy.to_string(),
        has_answers: poll.has_answer(user.id),
        can_answer_strongly: user.can_answer_strongly(),
        poll,
        not_answered,
        no_date_answered_with_yes,
        update_answers_uri: uri!(update_answers()),
        close_poll_uri: user
            .can_manage_poll()
            .then(|| uri!(super::close_poll_page())),
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
    options: impl Iterator<Item = &'a PollOption>,
    user: &User,
) -> Vec<OpenPollOptionsGroup> {
    options
        .filter(|o: &&PollOption| !o.has_veto() || user.can_answer_strongly())
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

#[derive(Debug, Serialize)]
struct OpenPoll {
    poll: Poll,
    option_groups: Vec<OpenPollOptionsGroup>,
    date_selection_strategy: String,
    has_answers: bool,
    can_answer_strongly: bool,
    no_date_answered_with_yes: Vec<User>,
    not_answered: Vec<User>,
    update_answers_uri: Origin<'static>,
    close_poll_uri: Option<Origin<'static>>,
}

#[derive(Debug, Serialize)]
struct OpenPollOptionsGroup {
    name: String,
    options: Vec<OpenPollOption>,
}

#[derive(Debug, Serialize)]
struct OpenPollOption {
    id: i64,
    #[serde(with = "time::serde::iso8601")]
    starts_at: OffsetDateTime,
    yes: bool,
    strong: bool,
    vetoed: bool,
    yes_answers: Vec<User>,
}

#[post("/poll", data = "<form>")]
pub(super) async fn update_answers(
    mut repository: Box<dyn Repository>,
    user: User,
    form: Form<AnswerUpdates>,
    poll: Open<Poll>,
) -> Result<Redirect, Debug<Error>> {
    repository
        .add_answers(apply_updates(&poll, &user, form.into_inner()))
        .await?;
    Ok(Redirect::to(uri!(open_poll_page())))
}

fn apply_updates(
    poll: &Poll,
    user: &User,
    updates: AnswerUpdates,
) -> Vec<(i64, Answer<(), UserId>)> {
    poll.options
        .iter()
        .map(|option| (option.id, get_answer(user, &updates, option)))
        .collect()
}

fn get_answer(user: &User, updates: &AnswerUpdates, option: &PollOption) -> Answer<(), UserId> {
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

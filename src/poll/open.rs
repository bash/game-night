use super::{rocket_uri_macro_poll_page, Open};
use super::{Answer, AnswerValue};
use crate::database::Repository;
use crate::poll::{Poll, PollOption};
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use anyhow::Error;
use itertools::Itertools as _;
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{post, uri, FromForm};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::collections::HashMap;
use time::{Month, OffsetDateTime};

pub(super) fn open_poll_page(page: PageBuilder<'_>, poll: Poll, user: User) -> Template {
    page.type_(PageType::Poll)
        .render("poll/open", to_open_poll(poll, &user))
}

fn to_open_poll(poll: Poll, user: &User) -> OpenPoll {
    OpenPoll {
        option_groups: to_open_poll_options(poll.options.iter(), user),
        date_selection_strategy: poll.strategy.to_string(),
        has_answers: has_answers(&poll, user),
        can_answer_strongly: user.can_answer_strongly(),
        poll,
    }
}

fn has_answers(poll: &Poll, user: &User) -> bool {
    poll.options.iter().any(|o| o.get_answer(user.id).is_some())
}

fn to_open_poll_options<'a>(
    options: impl Iterator<Item = &'a PollOption>,
    user: &User,
) -> Vec<OpenPollOptionsGroup> {
    options
        .group_by(|o| o.starts_at.month())
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
        ends_at: option.ends_at,
        yes,
        strong,
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
    #[serde(with = "time::serde::iso8601")]
    ends_at: OffsetDateTime,
    yes: bool,
    strong: bool,
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
    Ok(Redirect::to(uri!(poll_page())))
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

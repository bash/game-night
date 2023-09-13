use super::{rocket_uri_macro_poll_page, Open};
use super::{Answer, AnswerValue, Attendance};
use crate::database::Repository;
use crate::poll::{Poll, PollOption};
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use anyhow::Error;
use itertools::Itertools;
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
        poll,
    }
}

fn to_open_poll_options<'a>(
    options: impl Iterator<Item = &'a PollOption>,
    user: &User,
) -> Vec<OpenPollOptionsGroup> {
    options
        .group_by(|o| o.datetime.month())
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
    OpenPollOption {
        id: option.id,
        datetime: option.datetime,
        answer,
    }
}

#[derive(Debug, Serialize)]
struct OpenPoll {
    poll: Poll,
    option_groups: Vec<OpenPollOptionsGroup>,
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
    datetime: OffsetDateTime,
    answer: Option<AnswerValue>,
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
        .filter(|option| !is_answered_with_yes_and_required_attendance(option, user)) // required attendences cannot currently not be updated
        .map(|option| (option.id, get_answer(user, &updates, option)))
        .collect()
}

fn get_answer(user: &User, updates: &AnswerUpdates, option: &PollOption) -> Answer<(), UserId> {
    Answer {
        id: (),
        user: user.id,
        value: get_answer_value(updates, option.id),
    }
}

fn get_answer_value(updates: &AnswerUpdates, option_id: i64) -> AnswerValue {
    let yes = updates.options.get(&option_id).copied().unwrap_or_default();
    if yes {
        AnswerValue::yes(Attendance::Optional)
    } else {
        AnswerValue::No
    }
}

fn is_answered_with_yes_and_required_attendance(option: &PollOption, user: &User) -> bool {
    matches!(option.get_answer(user.id), Some(answer) if is_required(answer))
}

fn is_required(answer: &Answer) -> bool {
    matches!(
        answer.value,
        AnswerValue::Yes {
            attendance: Attendance::Required
        }
    )
}

#[derive(Debug, FromForm)]
pub(super) struct AnswerUpdates {
    options: HashMap<i64, bool>,
}

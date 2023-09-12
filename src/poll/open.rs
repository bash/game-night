use super::rocket_uri_macro_poll_page;
use crate::database::Repository;
use crate::poll::{Poll, PollOption};
use crate::template::{PageBuilder, PageType};
use crate::users::User;
use anyhow::Error;
use itertools::Itertools;
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{post, uri, FromForm};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::collections::HashMap;
use time::{Month, OffsetDateTime};

use super::{Answer, AnswerValue, Attendance};

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
    form: Form<UpdateAnswersData>,
) -> Result<Redirect, Debug<Error>> {
    for (option_id, answer) in form.into_inner().options {
        let value = if answer {
            AnswerValue::yes(Attendance::Optional)
        } else {
            AnswerValue::No
        };
        let answer = Answer {
            id: (),
            value,
            user: user.id,
        };
        repository.add_answer(option_id, answer).await?;
    }
    Ok(Redirect::to(uri!(poll_page())))
}

#[derive(Debug, FromForm)]
pub(super) struct UpdateAnswersData {
    options: HashMap<i64, bool>,
}

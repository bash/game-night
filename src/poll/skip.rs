use super::{Answer, AnswerValue, Open, Poll};
use crate::database::Repository;
use crate::template::PageBuilder;
use crate::uri;
use crate::users::{User, UserId};
use anyhow::Error;
use rocket::response::{Debug, Redirect};
use rocket::{get, post};
use rocket_dyn_templates::{context, Template};

#[get("/poll/skip", rank = 10)]
pub(super) fn skip_poll_page(user: User, page: PageBuilder, poll: Open<Poll>) -> Template {
    page.render(
        "poll/skip",
        context! {
            poll_uri: uri!(crate::poll::open_poll_page()),
            has_answers: poll.has_answer(user.id),
            poll,
        },
    )
}

#[get("/poll/skip", rank = 20)]
pub(super) fn skip_poll_fallback(user: User) -> Redirect {
    Redirect::to("/")
}

#[post("/poll/skip")]
pub(super) async fn skip_poll(
    user: User,
    poll: Open<Poll>,
    mut repository: Box<dyn Repository>,
) -> Result<Redirect, Debug<Error>> {
    let answers = get_no_answers(&user, &poll);
    repository.add_answers(answers).await?;
    Ok(Redirect::to(uri!(skip_poll_page)))
}

fn get_no_answers(user: &User, poll: &Poll) -> Vec<(i64, Answer<(), UserId>)> {
    poll.options
        .iter()
        .map(|option| {
            (
                option.id,
                Answer {
                    id: (),
                    user: user.id,
                    value: AnswerValue::no(false),
                },
            )
        })
        .collect()
}

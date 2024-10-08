use super::{Answer, AnswerValue, Poll};
use crate::database::{New, Repository};
use crate::event::EventsQuery;
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::users::User;
use crate::{responder, uri};
use rocket::response::Redirect;
use rocket::{get, post};
use rocket_dyn_templates::{context, Template};

#[get("/event/<id>/skip")]
pub(super) async fn skip_poll_page(
    id: i64,
    user: User,
    mut events: EventsQuery,
    page: PageBuilder<'_>,
) -> HttpResult<SkipPollResponse> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Ok(Redirect::to(uri!(crate::home_page())).into());
    };
    let ctx = context! {
        poll_uri: uri!(crate::event::event_page(id = poll.event.id)),
        has_answers: poll.has_answer(user.id),
        poll,
    };
    Ok(page.render("poll/skip", ctx).into())
}

responder! {
    pub(crate) enum SkipPollResponse {
        Redirect(Box<Redirect>),
        Template(Template),
    }
}

#[post("/event/<id>/skip")]
pub(super) async fn skip_poll(
    id: i64,
    user: User,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Ok(Redirect::to(uri!(crate::home_page())));
    };
    let answers = get_no_answers(&user, &poll);
    repository.add_answers(answers).await?;
    Ok(Redirect::to(uri!(skip_poll_page(id = poll.event.id))))
}

fn get_no_answers(user: &User, poll: &Poll) -> Vec<(i64, Answer<New>)> {
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

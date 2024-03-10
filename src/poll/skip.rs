use super::{Answer, AnswerValue, Open, Poll};
use crate::database::Repository;
use crate::uri;
use crate::users::{User, UserId};
use anyhow::Error;
use rocket::get;
use rocket::response::{Debug, Redirect};

// This is unfortunately a get route so that users
// from the email can directly skip the poll.
#[get("/poll/skip")]
pub(super) async fn skip_poll(
    user: User,
    poll: Open<Poll>,
    mut repository: Box<dyn Repository>,
) -> Result<Redirect, Debug<Error>> {
    let answers = get_no_answers(&user, &poll);
    repository.add_answers(answers).await?;
    Ok(Redirect::to(uri!(super::open::open_poll_page)))
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

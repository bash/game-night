use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::Repository;
use crate::event::EventsQuery;
use crate::login::RedirectUri;
use crate::poll::{finalize, NudgeFinalizer, PollStage};
use crate::result::HttpResult;
use crate::template::PageBuilder;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, uri, State};
use rocket_dyn_templates::{context, Template};

#[get("/event/<id>/poll/close")]
pub(crate) async fn close_poll_page(
    id: i64,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::NotFound.into());
    };
    let candidates = finalize::get_candidates(&poll);
    let close_manually = matches!(poll.stage, PollStage::Blocked);
    let set_close_manually_uri = uri!(super::set_close_manually(
        id = id,
        redirect_to = &uri!(close_poll_page(id = id))
    ));
    Ok(page.render(
        "poll/close",
        context! {
            date_selection_strategy: poll.strategy.to_string(),
            poll,
            candidates,
            close_manually,
            set_close_manually_uri,
        },
    ))
}

#[post("/event/<id>/poll/close")]
pub(crate) async fn close_poll(
    id: i64,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    nudge: &State<NudgeFinalizer>,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::BadRequest.into());
    };
    repository
        .update_poll_stage(poll.id, PollStage::Pending)
        .await?;
    nudge.nudge();
    let event_uri = uri!(crate::event::event_page(id = poll.event.id));
    Ok(Redirect::to(event_uri))
}

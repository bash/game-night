use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::{New, Repository};
use crate::event::EventsQuery;
use crate::login::RedirectUri;
use crate::poll::{
    finalize, Answer, AnswerValue, NudgeFinalizer, Poll, PollOption, PollOptionPatch, PollStage,
};
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::users::User;
use crate::users::UserNameComponent;
use anyhow::Result;
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, uri, FromForm, State};

#[get("/event/<id>/poll/close")]
pub(crate) async fn close_poll_page(
    id: i64,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    page: PageContextBuilder<'_>,
) -> HttpResult<Templated<ClosePollPage>> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::NotFound.into());
    };
    let candidates = finalize::get_candidates(&poll);
    let close_manually = matches!(poll.stage, PollStage::Blocked);
    let set_close_manually_uri = uri!(super::set_close_manually(
        id = id,
        redirect_to = &uri!(close_poll_page(id = id))
    ));
    let page = ClosePollPage {
        poll,
        candidates,
        close_manually,
        set_close_manually_uri,
        ctx: page.build(),
    };
    Ok(Templated(page))
}

#[derive(Template, Debug)]
#[template(path = "poll/close.html")]
pub(crate) struct ClosePollPage {
    poll: Poll,
    candidates: Vec<PollOption>,
    close_manually: bool,
    set_close_manually_uri: Origin<'static>,
    ctx: PageContext,
}

#[post("/event/<id>/poll/close", data = "<data>")]
pub(crate) async fn close_poll(
    id: i64,
    data: Form<ClosePollData>,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    nudge: &State<NudgeFinalizer>,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::BadRequest.into());
    };
    apply_actions(&user, &mut *repository, &data.actions).await?;
    repository
        .update_poll_stage(poll.event.id, PollStage::Pending)
        .await?;
    nudge.nudge();
    let event_uri = uri!(crate::event::event_page(id = poll.event.id));
    Ok(Redirect::to(event_uri))
}

async fn apply_actions(
    user: &User,
    repository: &mut dyn Repository,
    actions: &[PollOptionAction],
) -> Result<()> {
    for action in actions {
        apply_action(user, repository, action).await?;
    }
    Ok(())
}

async fn apply_action(
    user: &User,
    repository: &mut dyn Repository,
    action: &PollOptionAction,
) -> Result<()> {
    let patch = PollOptionPatch {
        promote: Some(action.promote),
    };
    repository.update_poll_option(action.id, patch).await?;
    if action.veto {
        let answer = Answer::<New> {
            id: (),
            value: AnswerValue::veto(),
            user: user.id,
        };
        repository.add_answers(vec![(action.id, answer)]).await?;
    }
    Ok(())
}

#[derive(Debug, FromForm)]
pub(crate) struct ClosePollData {
    actions: Vec<PollOptionAction>,
}

#[derive(Debug, FromForm)]
pub(crate) struct PollOptionAction {
    id: i64,
    promote: bool,
    veto: bool,
}

use crate::auth::{AuthorizedTo, ManagePoll};
use crate::event::EventsQuery;
use crate::login::RedirectUri;
use crate::poll::PollStage;
use crate::{HttpResult, Repository};
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{post, FromForm};

#[post("/event/<id>/poll/close-manually?<redirect_to>", data = "<data>")]
pub(crate) async fn set_close_manually(
    id: i64,
    redirect_to: RedirectUri,
    data: Form<CloseManuallyData>,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    if events
        .with_id(id, &user)
        .await?
        .and_then(|e| e.polling())
        .is_none()
    {
        return Err(Status::BadRequest.into());
    };
    repository
        .update_poll_stage(id, PollStage::from_close_manually(data.close_manually))
        .await?;
    Ok(Redirect::to(redirect_to.0))
}

#[derive(FromForm)]
pub(crate) struct CloseManuallyData {
    close_manually: bool,
}

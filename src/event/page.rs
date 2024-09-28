use super::EventId;
use crate::database::Repository;
use crate::users::User;
use anyhow::Error;
use rocket::get;
use rocket::response::Debug;

#[get("/event/<id>")]
pub(crate) async fn event_page(
    id: EventId,
    mut repository: Box<dyn Repository>,
    _user: User,
) -> Result<String, Debug<Error>> {
    let event = repository.get_stateful_event(id).await?;
    Ok(format!("{event:#?}"))
}

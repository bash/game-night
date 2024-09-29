use super::StatefulEvent;
use crate::database::Repository;
use crate::poll::open_poll_page;
use crate::template::PageBuilder;
use crate::users::User;
use anyhow::Error;
use rocket::get;
use rocket::response::Debug;
use rocket_dyn_templates::Template;
use StatefulEvent::*;

#[get("/event/<id>")]
pub(crate) async fn event_page(
    user: User,
    id: i64, // TODO: uri!() macro has trouble with type alias
    mut repository: Box<dyn Repository>,
    page: PageBuilder<'_>,
) -> Result<Template, Debug<Error>> {
    let event = repository.get_stateful_event(id).await?;
    match event {
        Some(Polling(poll)) => open_poll_page(user, poll, page, repository).await,
        _ => todo!(),
    }
}

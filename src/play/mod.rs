use crate::database::Repository;
use crate::template::{PageBuilder, PageType};
use crate::users::User;
use anyhow::Error;
use rocket::response::Debug;
use rocket::{get, routes, Route};
use rocket_dyn_templates::{context, Template};

pub(crate) fn routes() -> Vec<Route> {
    routes![play_page]
}

#[get("/play")]
async fn play_page(
    mut repository: Box<dyn Repository>,
    page: PageBuilder<'_>,
    _user: User,
) -> Result<Template, Debug<Error>> {
    let event = repository.get_next_event().await?;
    Ok(page
        .type_(PageType::Play)
        .render("play", context! { event }))
}

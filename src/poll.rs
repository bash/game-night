use crate::template::{PageBuilder, PageType};
use crate::users::User;
use rocket::{get, routes, Route};
use rocket_dyn_templates::{context, Template};

pub(crate) fn routes() -> Vec<Route> {
    routes![poll_page]
}

#[get("/poll")]
fn poll_page(page: PageBuilder<'_>, _user: User) -> Template {
    page.type_(PageType::Poll).render("poll", context! {})
}

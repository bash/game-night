use crate::authorization::{AuthorizedTo, ManagePoll};
use crate::template::{PageBuilder, PageType};
use crate::users::User;
use rocket::{get, routes, Route};
use rocket_dyn_templates::{context, Template};

pub(crate) fn routes() -> Vec<Route> {
    routes![poll_page, new_poll_page]
}

#[get("/poll")]
fn poll_page(page: PageBuilder<'_>, _user: User) -> Template {
    page.type_(PageType::Poll).render("poll", context! {})
}

#[get("/poll/new")]
fn new_poll_page(page: PageBuilder<'_>, _user: AuthorizedTo<ManagePoll>) -> Template {
    todo!()
}

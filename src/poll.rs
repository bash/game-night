use crate::authorization::{AuthorizedTo, ManagePoll};
use crate::template::{PageBuilder, PageType};
use crate::users::User;
use rocket::{get, routes, uri, Route};
use rocket_dyn_templates::{context, Template};

pub(crate) fn routes() -> Vec<Route> {
    routes![poll_page, new_poll_page]
}

#[get("/poll")]
fn poll_page(page: PageBuilder<'_>, user: User) -> Template {
    let new_poll_uri = user.can_manage_poll().then(|| uri!(new_poll_page()));
    page.type_(PageType::Poll)
        .render("poll", context! { new_poll_uri })
}

#[get("/poll/new")]
fn new_poll_page(page: PageBuilder<'_>, _user: AuthorizedTo<ManagePoll>) -> Template {
    todo!()
}

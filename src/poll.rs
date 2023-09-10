use crate::template::{PageBuilder, PageType};
use crate::users::User;
use rocket::{get, routes, uri, Route};
use rocket_dyn_templates::{context, Template};

mod new;

pub(crate) fn routes() -> Vec<Route> {
    routes![poll_page, new::new_poll_page, new::new_poll]
}

#[get("/poll")]
fn poll_page(page: PageBuilder<'_>, user: User) -> Template {
    let new_poll_uri = user.can_manage_poll().then(|| uri!(new::new_poll_page()));
    page.type_(PageType::Poll)
        .render("poll", context! { new_poll_uri })
}

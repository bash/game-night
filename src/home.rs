use crate::{uri, PageBuilder};
use rocket::get;
use rocket_dyn_templates::{context, Template};

#[get("/", rank = 20)]
pub(crate) fn home_page(page: PageBuilder<'_>) -> Template {
    page.render(
        "index",
        context! { getting_invited_uri: uri!(crate::register::getting_invited_page())},
    )
}

use crate::template_v2::prelude::*;
use crate::{uri, PageBuilder};
use rocket::get;
use rocket::http::uri::Origin;
use rocket::response::Responder;

#[get("/", rank = 20)]
pub(crate) fn home_page(page: PageBuilder<'_>) -> impl Responder {
    Templated(HomePage {
        getting_invited_uri: uri!(crate::register::getting_invited_page()),
        ctx: page.build(),
    })
}

#[derive(Template, Debug)]
#[template(path = "index.html")]
pub(crate) struct HomePage {
    pub(super) getting_invited_uri: Origin<'static>,
    pub(super) ctx: PageContext,
}

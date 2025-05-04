use crate::template::Chapter;
use crate::users::User;
use rocket::http::uri::Origin;

pub(crate) mod filters;
pub(crate) mod functions;
pub(crate) mod responder;

pub(crate) mod prelude {
    pub(crate) use super::responder::Templated;
    pub(crate) use super::{filters, functions, PageContext};
    pub(crate) use crate::template::PageBuilder;
    pub(crate) use askama::Template;
}

#[derive(Debug)]
pub(crate) struct PageContext {
    pub(crate) user: Option<User>,
    pub(crate) logout_uri: Origin<'static>,
    pub(crate) impersonating: bool,
    pub(crate) page: Page,
    pub(crate) chapters: Vec<Chapter>,
    pub(crate) active_chapter: Chapter,
    pub(crate) import_map: Option<String>,
}

impl PageContext {
    pub(crate) fn asset(&self, path: impl ToString) -> String {
        // TODO
        path.to_string()
    }
}

#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) uri: Origin<'static>,
    pub(crate) path: String,
}

#[cfg(debug_assertions)]
mod test_base {
    use super::prelude::*;
    #[derive(Template)]
    #[template(path = "base.html")]
    struct _BaseTemplate {
        ctx: PageContext,
    }
}

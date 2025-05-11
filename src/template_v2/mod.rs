pub(crate) mod convert;
pub(crate) mod filters;
pub(crate) mod functions;
pub(crate) mod page_context;
pub(crate) mod responder;

pub(crate) mod prelude {
    pub(crate) use super::page_context::{PageContext, PageContextBuilder};
    pub(crate) use super::responder::Templated;
    pub(crate) use super::{filters, functions};
    pub(crate) use askama::Template;
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

mod assets;
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

pub(crate) use fairing::template_fairing;

mod fairing {
    use super::assets::{Assets, SharedAssets};
    use rocket::error;
    use rocket::fairing::{self, Fairing};
    use std::sync::Arc;

    pub(crate) fn template_fairing() -> impl Fairing {
        fairing::AdHoc::try_on_ignite("Template Assets", |rocket| {
            Box::pin(async {
                match Assets::load() {
                    Ok(assets) => Ok(rocket.manage(SharedAssets(Arc::new(assets)))),
                    Err(error) => {
                        error!("failed to initialize template assets:\n{:?}", error);
                        Err(rocket)
                    }
                }
            })
        })
    }
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

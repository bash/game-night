use crate::result::HttpResult;
use crate::template_v2::prelude::*;
use anyhow::anyhow;
use rocket::request::FromRequest as _;
use rocket::{catch, catchers, Catcher, Request};

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![not_found, forbidden]
}

#[catch(404)]
async fn not_found(request: &Request<'_>) -> HttpResult<Templated<NotFoundPage>> {
    let ctx = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?
        .build();
    Ok(Templated(NotFoundPage { ctx }))
}

#[catch(403)]
async fn forbidden(request: &Request<'_>) -> HttpResult<Templated<ForbiddenPage>> {
    let ctx = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?
        .build();
    Ok(Templated(ForbiddenPage { ctx }))
}

#[derive(Template)]
#[template(path = "errors/404.html")]
pub(super) struct NotFoundPage {
    pub(super) ctx: PageContext,
}

#[derive(Template)]
#[template(path = "errors/403.html")]
pub(super) struct ForbiddenPage {
    pub(super) ctx: PageContext,
}

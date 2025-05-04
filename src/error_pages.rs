use crate::{HttpResult, PageBuilder};
use anyhow::anyhow;
use rocket::request::FromRequest as _;
use rocket::{catch, catchers, Catcher, Request};
use rocket_dyn_templates::Template;

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![not_found, forbidden]
}

#[catch(404)]
async fn not_found(request: &Request<'_>) -> HttpResult<Template> {
    let page = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?;
    Ok(page.render("errors/404", ()))
}

#[catch(403)]
async fn forbidden(request: &Request<'_>) -> HttpResult<Template> {
    let page = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?;
    Ok(page.render("errors/403", ()))
}

use crate::auth::LoginState;
use crate::login::rocket_uri_macro_logout;
use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Origin;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, uri, Request};
use rocket_dyn_templates::{Engines, Template};
use serde::Serialize;
use std::borrow::Cow;

#[macro_use]
mod macros;
mod functions;
pub(crate) use functions::*;

pub(crate) struct PageBuilder<'r> {
    user: Option<User>,
    login_state: LoginState,
    uri: &'r Origin<'r>,
}

impl<'r> PageBuilder<'r> {
    pub(crate) fn render(
        &self,
        name: impl Into<Cow<'static, str>>,
        context: impl Serialize,
    ) -> Template {
        Template::render(
            name,
            TemplateContext {
                context,
                user: self.user.as_ref(),
                logout_uri: uri!(logout()),
                sudo: self.login_state.is_sudo(),
                page: Page {
                    uri: self.uri,
                    path: self.uri.path().as_str(),
                },
            },
        )
    }
}

#[derive(Serialize)]
struct TemplateContext<'a, C>
where
    C: Serialize,
{
    user: Option<&'a User>,
    logout_uri: Origin<'a>,
    sudo: bool,
    page: Page<'a>,
    #[serde(flatten)]
    context: C,
}

#[derive(Serialize)]
struct Page<'a> {
    uri: &'a Origin<'a>,
    path: &'a str,
}

#[async_trait]
impl<'r> FromRequest<'r> for PageBuilder<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user: Option<User> = request
            .guard()
            .await
            .expect("Option<T> guard is infallible");
        let login_state: LoginState = try_outcome!(request.guard().await);
        let uri = request.uri();
        Outcome::Success(PageBuilder {
            user,
            uri,
            login_state,
        })
    }
}

pub(crate) fn configure_template_engines(engines: &mut Engines) {
    functions::register_custom_functions(&mut engines.tera);
}

use crate::auth::LoginState;
use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Origin;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
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
    type_: PageType,
}

impl<'r> PageBuilder<'r> {
    pub(crate) fn type_(mut self, type_: PageType) -> Self {
        self.type_ = type_;
        self
    }

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
                sudo: self.login_state.is_sudo(),
                page: Page {
                    uri: self.uri,
                    path: self.uri.path().as_str(),
                    type_: self.type_,
                    chapter_number: self.type_.chapter_number(),
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
    sudo: bool,
    page: Page<'a>,
    #[serde(flatten)]
    context: C,
}

#[derive(Serialize)]
struct Page<'a> {
    uri: &'a Origin<'a>,
    path: &'a str,
    #[serde(rename = "type")]
    type_: PageType,
    chapter_number: &'a str,
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
            type_: PageType::Home,
            login_state,
        })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PageType {
    #[default]
    Home,
    Invite,
    Register,
    Poll,
    Play,
}

impl PageType {
    fn chapter_number(self) -> &'static str {
        use PageType::*;
        match self {
            Home => "Zero",
            Invite => "One",
            Register => "Two",
            Poll => "Three",
            Play => "Four",
        }
    }
}

impl<'a, 'r> TryFrom<&'a Origin<'r>> for PageType {
    type Error = ();

    fn try_from(value: &'a Origin<'r>) -> Result<Self, Self::Error> {
        use PageType::*;
        match value.path().segments().get(0) {
            None => Ok(Home),
            Some("invite") => Ok(Invite),
            Some("register") => Ok(Register),
            Some("poll") => Ok(Poll),
            Some("play") => Ok(Play),
            _ => Err(()),
        }
    }
}

impl<'r> TryFrom<Origin<'r>> for PageType {
    type Error = ();

    fn try_from(value: Origin<'r>) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

pub(crate) fn configure_template_engines(engines: &mut Engines) {
    functions::register_custom_functions(&mut engines.tera);
}

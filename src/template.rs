use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Origin;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::borrow::Cow;

pub(crate) struct PageBuilder<'r> {
    user: Option<User>,
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
            context! {
                user: &self.user,
                uri: self.uri,
                path: self.uri.path().as_str(),
                page_type: self.type_,
                chapter_number: self.type_.chapter_number(),
                page: context
            },
        )
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for PageBuilder<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user: Option<User> = request
            .guard()
            .await
            .expect("Option<T> guard is infallible");
        let uri = request.uri();
        Outcome::Success(PageBuilder {
            user,
            uri,
            type_: PageType::Home,
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

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
        let user: Option<User> = request.guard().await.unwrap();
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

impl<'r> TryFrom<Origin<'r>> for PageType {
    type Error = ();

    fn try_from(value: Origin<'r>) -> Result<Self, Self::Error> {
        match value.path().segments().get(0) {
            None => Ok(Self::Home),
            Some("invite") => Ok(Self::Invite),
            Some("register") => Ok(Self::Register),
            Some("poll") => Ok(Self::Poll),
            Some("play") => Ok(Self::Play),
            _ => Err(()),
        }
    }
}

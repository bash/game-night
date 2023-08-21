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
            context! { user: &self.user, uri: self.uri, page_type: self.type_, page: context },
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

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PageType {
    Home,
    Invite,
    Register,
    Poll,
    Play,
}

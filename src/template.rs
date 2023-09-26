use crate::users::User;
use anyhow::Error;
use rocket::http::uri::Origin;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use rocket_dyn_templates::{Engines, Template};
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::iter;

#[macro_use]
mod macros;

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
            TemplateContext {
                context,
                user: self.user.as_ref(),
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

pub(crate) fn configure_template_engines(engines: &mut Engines) {
    engines.tera.register_filter("markdown", markdown_filter);
    engines.tera.register_function("ps", ps_prefix);
}

tera_function! {
    fn ps_prefix(level: usize = 0) {
        Ok(tera::Value::String(
            iter::repeat("P.")
                .take(level + 1)
                .chain(iter::once("S."))
                .collect(),
        ))
    }
}

fn markdown_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    use pulldown_cmark::{html, Options, Parser};

    const OPTIONS: Options = Options::empty()
        .union(Options::ENABLE_TABLES)
        .union(Options::ENABLE_FOOTNOTES)
        .union(Options::ENABLE_STRIKETHROUGH);

    let input = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("This filter expects a string as input"))?;

    let parser = Parser::new_ext(input, OPTIONS);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(html_output.into())
}

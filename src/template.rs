use crate::auth::LoginState;
use crate::login::rocket_uri_macro_logout;
use crate::users::User;
use anyhow::Error;
use itertools::Itertools;
use lazy_static::lazy_static;
use rocket::http::uri::Origin;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, uri, Request};
use rocket_dyn_templates::{Engines, Template};
use serde::Serialize;
use std::borrow::Cow;
use std::cmp;

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
        let chapters = visible_chapters(&self.user);
        Template::render(
            name,
            TemplateContext {
                context,
                user: self.user.as_ref(),
                logout_uri: uri!(logout()),
                sudo: self.login_state.is_sudo(),
                active_chapter: active_chapter(&chapters, self.uri),
                chapters,
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
    chapters: Vec<Chapter>,
    active_chapter: Chapter,
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

fn visible_chapters(user: &Option<User>) -> Vec<Chapter> {
    CHAPTERS
        .iter()
        .filter(|b| (b.visible_if)(user))
        .cloned()
        .collect()
}

fn active_chapter(chapters: &[Chapter], uri: &Origin<'_>) -> Chapter {
    chapters
        .iter()
        .filter(|c| uri.path().starts_with(c.uri.path().as_str()))
        .sorted_by_key(|c| cmp::Reverse(c.uri.path().segments().len()))
        .next()
        .cloned()
        .expect("Root chapter should always match")
}

lazy_static! {
    static ref CHAPTERS: Vec<Chapter> = vec![
        Chapter {
            uri: Origin::ROOT,
            title: "Home",
            visible_if: |_| true,
            accent_color: AccentColor::Purple,
            icon: SvgIcon {
                name: "home",
                aria_label: "Home"
            }
        },
        Chapter {
            uri: uri!("/invite"),
            title: "Get Invited",
            visible_if: |_| true,
            accent_color: AccentColor::Red,
            icon: SvgIcon {
                name: "mail-open",
                aria_label: "Mail"
            }
        },
        Chapter {
            uri: uri!("/register"),
            title: "Register",
            accent_color: AccentColor::Green,
            visible_if: |_| true,
            icon: SvgIcon {
                name: "clipboard-signature",
                aria_label: "Clipboard Signature"
            }
        },
        Chapter {
            uri: uri!("/poll"),
            title: "Poll",
            visible_if: |_| true,
            accent_color: AccentColor::Teal,
            icon: SvgIcon {
                name: "calendar-check",
                aria_label: "Calendar Check"
            }
        },
        Chapter {
            uri: uri!("/play"),
            title: "Play",
            visible_if: |_| true,
            accent_color: AccentColor::Blue,
            icon: SvgIcon {
                name: "dices",
                aria_label: "Dices"
            }
        },
    ];
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Chapter {
    uri: Origin<'static>,
    title: &'static str,
    #[serde(skip)]
    visible_if: fn(&Option<User>) -> bool,
    accent_color: AccentColor,
    icon: SvgIcon,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SvgIcon {
    name: &'static str,
    aria_label: &'static str,
}

#[derive(Debug, Copy, Clone, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccentColor {
    #[default]
    Purple,
    Blue,
    Teal,
    Green,
    Red,
}

impl AccentColor {
    pub(crate) fn as_slice() -> &'static [AccentColor] {
        use AccentColor::*;
        &[Purple, Blue, Teal, Green, Red]
    }

    pub(crate) fn css_value(self) -> &'static str {
        use AccentColor::*;
        match self {
            Purple => "var(--purple-color)",
            Blue => "var(--blue-color)",
            Teal => "var(--teal-color)",
            Green => "var(--green-color)",
            Red => "var(--red-color)",
        }
    }
}

pub(crate) fn configure_template_engines(engines: &mut Engines) {
    functions::register_custom_functions(&mut engines.tera);
}

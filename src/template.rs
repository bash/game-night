use crate::auth::LoginState;
use crate::login::rocket_uri_macro_login;
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
        .filter(|c| path_matches(uri, &c.uri) || c.match_uris.iter().any(|u| path_matches(uri, u)))
        .sorted_by_key(|c| cmp::Reverse(c.uri.path().segments().len()))
        .next()
        .cloned()
        .expect("Root chapter should always match")
}

fn path_matches(uri: &Origin<'_>, expected_prefix: &Origin<'_>) -> bool {
    uri.path().starts_with(expected_prefix.path().as_str())
}

lazy_static! {
    static ref CHAPTERS: Vec<Chapter> = vec![
        Chapter {
            uri: Origin::ROOT,
            match_uris: vec![],
            title: "Register",
            visible_if: Option::is_none,
            accent_color: AccentColor::Purple,
            icon: SvgIcon {
                name: "clipboard-signature",
                aria_label: "Clipboard Signature"
            }
        },
        Chapter {
            uri: uri!(login(redirect = Some("/"))),
            match_uris: vec![],
            title: "Play",
            visible_if: Option::is_none,
            accent_color: AccentColor::Blue,
            icon: SvgIcon {
                name: "dices",
                aria_label: "Dices"
            }
        },
        Chapter {
            uri: Origin::ROOT,
            match_uris: vec![],
            title: "Play",
            visible_if: Option::is_some,
            accent_color: AccentColor::Blue,
            icon: SvgIcon {
                name: "dices",
                aria_label: "Dices"
            }
        },
        Chapter {
            uri: uri!("/profile"),
            match_uris: vec![uri!("/users")],
            title: "User Profile",
            accent_color: AccentColor::Teal,
            visible_if: |_| true,
            icon: SvgIcon {
                name: "user",
                aria_label: "User"
            }
        },
        Chapter {
            uri: uri!("/news"),
            match_uris: vec![],
            title: "News",
            visible_if: |_| true,
            accent_color: AccentColor::Green,
            icon: SvgIcon {
                name: "megaphone",
                aria_label: "Megaphone"
            }
        },
        Chapter {
            uri: uri!("/invite"),
            match_uris: vec![],
            title: "Invite",
            visible_if: |u| u.as_ref().map(|u| u.can_invite()).unwrap_or_default(),
            accent_color: AccentColor::Red,
            icon: SvgIcon {
                name: "mail-open",
                aria_label: "Mail"
            }
        },
    ];
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Chapter {
    uri: Origin<'static>,
    match_uris: Vec<Origin<'static>>,
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

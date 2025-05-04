use crate::auth::LoginState;
use crate::invitation::rocket_uri_macro_invite_page;
use crate::invitation::Passphrase;
use crate::login::rocket_uri_macro_logout;
use crate::play::{rocket_uri_macro_archive_page, rocket_uri_macro_play_redirect};
use crate::register::{rocket_uri_macro_profile, rocket_uri_macro_register_page};
use crate::users::rocket_uri_macro_list_users;
use crate::users::User;
use anyhow::Error;
use itertools::Itertools;
use rocket::http::uri::Origin;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, uri, Request};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::OnceLock;

use rocket::http::ext::IntoOwned;
use std::convert::Infallible;

pub(crate) struct PageBuilder<'r> {
    user: Option<User>,
    login_state: LoginState,
    uri: Cow<'r, Origin<'r>>,
}

impl PageBuilder<'_> {
    pub(crate) fn uri(mut self, uri: Option<impl Into<Origin<'static>>>) -> Self {
        if let Some(uri) = uri {
            self.uri = Cow::Owned(uri.into());
        }
        self
    }

    pub(crate) fn build(self) -> crate::template_v2::PageContext {
        use crate::template_v2::{Page, PageContext};
        let chapters = visible_chapters(&self.user);
        PageContext {
            user: self.user,
            logout_uri: uri!(logout()),
            impersonating: self.login_state.is_impersonating(),
            active_chapter: active_chapter(&chapters, &self.uri),
            chapters,
            import_map: None, // TODO
            page: Page {
                uri: self.uri.clone().into_owned().into_owned(),
                path: self.uri.path().raw().percent_decode_lossy().into_owned(),
            },
        }
    }

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
                sudo: self.login_state.is_impersonating(),
                active_chapter: active_chapter(&chapters, &self.uri),
                chapters,
                page: Page {
                    uri: &self.uri,
                    path: &self.uri.path().raw().percent_decode_lossy(),
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
        let user: Option<User> = unwrap_infallible(request.guard().await);
        let login_state: LoginState = try_outcome!(request.guard().await);
        let uri = Cow::Borrowed(request.uri());
        Outcome::Success(PageBuilder {
            user,
            uri,
            login_state,
        })
    }
}

fn unwrap_infallible<S>(outcome: Outcome<S, Infallible>) -> S {
    match outcome {
        Outcome::Success(value) => value,
        Outcome::Forward(status) => panic!(
            "unexpectedly got a forward from an infallible guard: {:?}",
            status
        ),
    }
}

fn visible_chapters(user: &Option<User>) -> Vec<Chapter> {
    chapters()
        .iter()
        .filter(|b| (b.visible_if)(user))
        .cloned()
        .collect()
}

fn active_chapter(chapters: &[Chapter], uri: &Origin<'_>) -> Chapter {
    chapters
        .iter()
        .filter(|c| path_matches(uri, &c.uri) || c.match_uris.iter().any(|u| path_matches(uri, u)))
        .sorted_by_key(|c| c.weight)
        .next()
        .cloned()
        .expect("Root chapter should always match")
}

fn path_matches(uri: &Origin<'_>, expected_prefix: &Origin<'_>) -> bool {
    uri.path().starts_with(expected_prefix.path().as_str())
}

fn chapters() -> &'static [Chapter] {
    static CHAPTERS: OnceLock<Vec<Chapter>> = OnceLock::new();
    CHAPTERS.get_or_init(|| {
        vec![
            Chapter {
                uri: uri!("/"),
                weight: 100,
                match_uris: vec![uri!(register_page(passphrase = Option::<Passphrase>::None))],
                title: "Register",
                visible_if: Option::is_none,
                accent_color: AccentColor::Pink,
                icon: SvgIcon {
                    name: "clipboard-signature",
                    aria_label: "Clipboard Signature",
                },
            },
            Chapter {
                uri: uri!(play_redirect()),
                weight: 0,
                match_uris: vec![],
                title: "Play",
                visible_if: Option::is_none,
                accent_color: AccentColor::Purple,
                icon: SvgIcon {
                    name: "dices",
                    aria_label: "Dices",
                },
            },
            Chapter {
                uri: uri!(crate::event::events_entry_page()),
                weight: 100,
                match_uris: vec![uri!(archive_page())],
                title: "Play",
                visible_if: Option::is_some,
                accent_color: AccentColor::Purple,
                icon: SvgIcon {
                    name: "dices",
                    aria_label: "Dices",
                },
            },
            Chapter {
                uri: uri!(profile()),
                weight: 0,
                match_uris: vec![uri!(list_users())],
                title: "User Profile",
                accent_color: AccentColor::Blue,
                visible_if: |_| true,
                icon: SvgIcon {
                    name: "user",
                    aria_label: "User",
                },
            },
            #[cfg(debug_assertions)]
            Chapter {
                uri: uri!("/news"),
                weight: 0,
                match_uris: vec![],
                title: "News",
                visible_if: |_| true,
                accent_color: AccentColor::Green,
                icon: SvgIcon {
                    name: "megaphone",
                    aria_label: "Megaphone",
                },
            },
            Chapter {
                uri: uri!(invite_page()),
                weight: 0,
                match_uris: vec![],
                title: "Invite",
                visible_if: |u| u.as_ref().map(|u| u.can_invite()).unwrap_or_default(),
                accent_color: AccentColor::Orange,
                icon: SvgIcon {
                    name: "mail-open",
                    aria_label: "Mail",
                },
            },
        ]
    })
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Chapter {
    pub(crate) uri: Origin<'static>,
    match_uris: Vec<Origin<'static>>,
    weight: usize,
    pub(crate) title: &'static str,
    #[serde(skip)]
    visible_if: fn(&Option<User>) -> bool,
    pub(crate) accent_color: AccentColor,
    pub(crate) icon: SvgIcon,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SvgIcon {
    pub(crate) name: &'static str,
    pub(crate) aria_label: &'static str,
}

#[derive(Debug, Copy, Clone, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccentColor {
    #[default]
    Pink,
    Purple,
    Blue,
    Green,
    Orange,
}

impl AccentColor {
    pub(crate) fn values() -> &'static [AccentColor] {
        use AccentColor::*;
        &[Pink, Purple, Blue, Green, Orange]
    }

    pub(crate) fn css_value(self) -> &'static str {
        use AccentColor::*;
        match self {
            Pink => "var(--pink-color)",
            Purple => "var(--purple-color)",
            Blue => "var(--blue-color)",
            Green => "var(--green-color)",
            Orange => "var(--orange-color)",
        }
    }

    pub(crate) fn css_class(self) -> &'static str {
        use AccentColor::*;
        match self {
            Pink => "_pink",
            Purple => "_purple",
            Blue => "_blue",
            Green => "_green",
            Orange => "_orange",
        }
    }
}

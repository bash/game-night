use super::assets::Assets;
use crate::auth::LoginState;
use crate::invitation::Passphrase;
use crate::uri;
use crate::users::User;
use itertools::Itertools as _;
use rocket::http::ext::IntoOwned as _;
use rocket::http::uri::Origin;
use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub(crate) struct PageContext {
    pub(crate) user: Option<User>,
    pub(crate) logout_uri: Origin<'static>,
    pub(crate) impersonating: bool,
    pub(crate) page: Page,
    pub(crate) chapters: Vec<Chapter>,
    pub(crate) active_chapter: Chapter,
    assets: Arc<Assets>,
}

impl PageContext {
    pub(crate) fn asset(&self, path: impl ToString) -> String {
        let path = path.to_string();
        self.assets.asset_map.get(&path).cloned().unwrap_or(path)
    }

    pub(crate) fn import_map(&self) -> Option<&str> {
        self.assets.import_map.as_deref()
    }
}

#[derive(Debug)]
pub(crate) struct Page {
    pub(crate) uri: Origin<'static>,
    pub(crate) path: String,
}

pub(crate) struct PageContextBuilder<'r> {
    user: Option<User>,
    login_state: LoginState,
    uri: Cow<'r, Origin<'r>>,
    assets: Arc<Assets>,
}

impl PageContextBuilder<'_> {
    pub(crate) fn uri(mut self, uri: Option<impl Into<Origin<'static>>>) -> Self {
        if let Some(uri) = uri {
            self.uri = Cow::Owned(uri.into());
        }
        self
    }

    pub(crate) fn build(self) -> PageContext {
        let chapters = visible_chapters(&self.user);
        PageContext {
            user: self.user,
            logout_uri: uri!(crate::login::logout()),
            impersonating: self.login_state.is_impersonating(),
            active_chapter: active_chapter(&chapters, &self.uri),
            chapters,
            page: Page {
                path: self.uri.path().raw().percent_decode_lossy().into_owned(),
                uri: self.uri.into_owned().into_owned(),
            },
            assets: self.assets,
        }
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
                match_uris: vec![uri!(crate::register::register_page(
                    passphrase = Option::<Passphrase>::None
                ))],
                title: "Register",
                visible_if: Option::is_none,
                accent_color: AccentColor::Pink,
                icon: SvgIcon {
                    name: "clipboard-signature",
                    aria_label: "Clipboard Signature",
                },
            },
            Chapter {
                uri: uri!(crate::play::play_redirect()),
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
                match_uris: vec![uri!(crate::play::archive_page())],
                title: "Play",
                visible_if: Option::is_some,
                accent_color: AccentColor::Purple,
                icon: SvgIcon {
                    name: "dices",
                    aria_label: "Dices",
                },
            },
            Chapter {
                uri: uri!(crate::register::profile()),
                weight: 0,
                match_uris: vec![uri!(crate::users::list_users())],
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
                uri: uri!(crate::invitation::invite_page()),
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

#[derive(Debug, Clone)]
pub(crate) struct Chapter {
    pub(crate) uri: Origin<'static>,
    match_uris: Vec<Origin<'static>>,
    weight: usize,
    pub(crate) title: &'static str,
    visible_if: fn(&Option<User>) -> bool,
    pub(crate) accent_color: AccentColor,
    pub(crate) icon: SvgIcon,
}

#[derive(Debug, Clone)]
pub(crate) struct SvgIcon {
    pub(crate) name: &'static str,
    pub(crate) aria_label: &'static str,
}

#[derive(Debug, Copy, Clone, Default)]
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

mod from_request {
    use super::PageContextBuilder;
    use crate::auth::LoginState;
    use crate::template::assets::SharedAssets;
    use crate::users::User;
    use anyhow::Error;
    use rocket::outcome::try_outcome;
    use rocket::request::{FromRequest, Outcome};
    use rocket::{async_trait, Request};
    use std::borrow::Cow;
    use std::convert::Infallible;

    #[async_trait]
    impl<'r> FromRequest<'r> for PageContextBuilder<'r> {
        type Error = Error;

        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
            let user: Option<User> = unwrap_infallible(request.guard().await);
            let login_state: LoginState = try_outcome!(request.guard().await);
            let assets: SharedAssets = try_outcome!(request.guard().await);
            let uri = Cow::Borrowed(request.uri());
            Outcome::Success(PageContextBuilder {
                user,
                uri,
                login_state,
                assets: assets.0,
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
}

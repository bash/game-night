use crate::users::{User, UserId, UserQueries};
use anyhow::{Error, Result};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::{try_outcome, IntoOutcome};
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use std::borrow::Cow;
use std::sync::Arc;

#[async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = Arc<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .local_cache_async(async {
                let mut users = try_outcome!(request
                    .guard::<UserQueries>()
                    .await
                    .map_error(|(status, e)| (status, Arc::new(e))));
                match fetch_user(request, &mut users).await {
                    Ok(Some(user)) => Outcome::Success(user),
                    Ok(None) => Outcome::Forward(Status::Unauthorized),
                    Err(e) => Outcome::Error((Status::InternalServerError, Arc::new(e))),
                }
            })
            .await
            .clone()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for LoginState {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .cookies()
            .login_state()
            .or_error(Status::InternalServerError)
    }
}

async fn fetch_user(request: &Request<'_>, users: &mut UserQueries) -> Result<Option<User>> {
    match request.cookies().login_state()?.effective_user_id() {
        Some(user_id) => Ok(users.by_id(user_id).await?.map(|u| u.to_v1())),
        None => Ok(None),
    }
}

pub(crate) trait CookieJarExt {
    fn login_state(&self) -> Result<LoginState>;

    fn set_login_state(&self, state: LoginState);
}

#[derive(Debug)]
pub(crate) enum LoginState {
    Authenticated(UserId),
    Impersonating { effective: UserId, original: UserId },
    Anonymous,
}

impl LoginState {
    pub(crate) fn effective_user_id(&self) -> Option<UserId> {
        match self {
            LoginState::Authenticated(user) => Some(*user),
            LoginState::Impersonating { effective, .. } => Some(*effective),
            LoginState::Anonymous => None,
        }
    }

    pub(crate) fn is_impersonating(&self) -> bool {
        matches!(self, LoginState::Impersonating { .. })
    }

    pub(crate) fn impersonate(self, effective: UserId) -> LoginState {
        use LoginState::*;
        match self {
            Authenticated(original) => Impersonating {
                effective,
                original,
            },
            Impersonating { original, .. } => Impersonating {
                effective,
                original,
            },
            Anonymous => Anonymous,
        }
    }
}

impl CookieJarExt for CookieJar<'_> {
    fn login_state(&self) -> Result<LoginState> {
        let effective = parse_user_id_cookie(self.get_private(USER_ID_COOKIE_NAME))?;
        let original = parse_user_id_cookie(self.get_private(ORIGINAL_USER_ID_COOKIE_NAME))?;
        match (effective, original) {
            (None, _) => Ok(LoginState::Anonymous),
            (Some(effective), None) => Ok(LoginState::Authenticated(effective)),
            (Some(effective), Some(original)) => Ok(LoginState::Impersonating {
                effective,
                original,
            }),
        }
    }

    fn set_login_state(&self, state: LoginState) {
        match state {
            LoginState::Anonymous => {
                self.remove_private(user_id_cookie(USER_ID_COOKIE_NAME, ""));
                self.remove_private(user_id_cookie(ORIGINAL_USER_ID_COOKIE_NAME, ""));
            }
            LoginState::Impersonating {
                effective,
                original,
            } => {
                self.add_private(user_id_cookie(USER_ID_COOKIE_NAME, effective.0.to_string()));
                self.add_private(user_id_cookie(
                    ORIGINAL_USER_ID_COOKIE_NAME,
                    original.0.to_string(),
                ));
            }
            LoginState::Authenticated(user) => {
                self.add_private(user_id_cookie(USER_ID_COOKIE_NAME, user.0.to_string()));
                self.remove_private(user_id_cookie(ORIGINAL_USER_ID_COOKIE_NAME, ""));
            }
        }
    }
}

fn parse_user_id_cookie(cookie: Option<Cookie>) -> Result<Option<UserId>> {
    Ok(cookie.map(|c| c.value().parse()).transpose()?.map(UserId))
}

fn user_id_cookie<'a>(name: &'a str, value: impl Into<Cow<'a, str>>) -> impl Into<Cookie<'a>> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(true)
        .permanent()
        .same_site(SameSite::Lax)
}

const USER_ID_COOKIE_NAME: &str = "user-id";
const ORIGINAL_USER_ID_COOKIE_NAME: &str = "original-user-id";

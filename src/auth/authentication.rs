use crate::database::Repository;
use crate::users::{User, UserId};
use anyhow::{Error, Result};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use std::borrow::Cow;
use std::ops::Deref;

pub(crate) struct UserGuard(User);

impl Deref for UserGuard {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = Option<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut repository = try_outcome!(request
            .guard::<Box<dyn Repository>>()
            .await
            .map_failure(|(s, e)| (s, Some(e))));
        match fetch_user(request, repository.as_mut()).await {
            Ok(Some(user)) => Outcome::Success(user),
            Ok(None) => Outcome::Failure((Status::Unauthorized, None)),
            Err(e) => Outcome::Failure((Status::InternalServerError, Some(e))),
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for LoginState {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.cookies().login_state() {
            Ok(s) => Outcome::Success(s),
            Err(e) => Outcome::Failure((Status::InternalServerError, e)),
        }
    }
}

async fn fetch_user(
    request: &Request<'_>,
    repository: &mut dyn Repository,
) -> Result<Option<User>> {
    match request.cookies().login_state()? {
        LoginState::Authenticated(e, _) => Ok(repository.get_user_by_id(e).await?),
        LoginState::Anonymous => Ok(None),
    }
}

pub(crate) trait CookieJarExt {
    fn login_state(&self) -> Result<LoginState>;

    fn set_login_state(&self, state: LoginState);
}

#[derive(Debug)]
pub(crate) enum LoginState {
    Authenticated(UserId, Option<UserId>),
    Anonymous,
}

impl LoginState {
    pub(crate) fn is_sudo(&self) -> bool {
        matches!(self, LoginState::Authenticated(_, Some(_)))
    }
}

impl<'r> CookieJarExt for CookieJar<'r> {
    fn login_state(&self) -> Result<LoginState> {
        let effective = parse_user_id_cookie(self.get_private(USER_ID_COOKIE_NAME))?;
        let original = parse_user_id_cookie(self.get_private(ORIGINAL_USER_ID_COOKIE_NAME))?;
        match effective {
            None => Ok(LoginState::Anonymous),
            Some(u) => Ok(LoginState::Authenticated(u, original)),
        }
    }

    fn set_login_state(&self, state: LoginState) {
        match state {
            LoginState::Anonymous => {
                self.remove_private(user_id_cookie(USER_ID_COOKIE_NAME, ""));
                self.remove_private(user_id_cookie(ORIGINAL_USER_ID_COOKIE_NAME, ""));
            }
            LoginState::Authenticated(e, Some(o)) => {
                self.add_private(user_id_cookie(USER_ID_COOKIE_NAME, e.0.to_string()));
                self.add_private(user_id_cookie(
                    ORIGINAL_USER_ID_COOKIE_NAME,
                    o.0.to_string(),
                ));
            }
            LoginState::Authenticated(e, None) => {
                self.add_private(user_id_cookie(USER_ID_COOKIE_NAME, e.0.to_string()));
                self.remove_private(user_id_cookie(ORIGINAL_USER_ID_COOKIE_NAME, ""));
            }
        }
    }
}

fn parse_user_id_cookie(cookie: Option<Cookie>) -> Result<Option<UserId>> {
    Ok(cookie.map(|c| c.value().parse()).transpose()?.map(UserId))
}

fn user_id_cookie<'a>(name: &'a str, value: impl Into<Cow<'a, str>>) -> Cookie<'a> {
    Cookie::build(name, value)
        .http_only(true)
        .secure(true)
        .permanent()
        .same_site(SameSite::Strict)
        .finish()
}

const USER_ID_COOKIE_NAME: &str = "user-id";
const ORIGINAL_USER_ID_COOKIE_NAME: &str = "original-user-id";

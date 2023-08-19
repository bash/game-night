use crate::database::Repository;
use crate::users::{User, UserId};
use anyhow::{Error, Result};
use rocket::http::private::cookie::CookieBuilder;
use rocket::http::{CookieJar, Status};
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use std::ops::Deref;

pub(crate) struct UserGuard(User);

impl UserGuard {
    pub(crate) fn into_inner(self) -> User {
        self.0
    }
}

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
            .map_failure(|(s, e)| (s, Some(e.into()))));
        match fetch_user(request, repository.as_mut()).await {
            Ok(Some(user)) => Outcome::Success(user),
            Ok(None) => Outcome::Failure((Status::Unauthorized, None)),
            Err(e) => Outcome::Failure((Status::InternalServerError, Some(e))),
        }
    }
}

async fn fetch_user(
    request: &Request<'_>,
    repository: &mut dyn Repository,
) -> Result<Option<User>> {
    match request.cookies().user_id()? {
        Some(i) => Ok(repository.get_user_by_id(i).await?),
        None => Ok(None),
    }
}

pub(crate) trait CookieJarExt {
    fn user_id(&self) -> Result<Option<UserId>>;

    fn set_user_id(&self, user_id: UserId);
}

impl<'r> CookieJarExt for CookieJar<'r> {
    fn user_id(&self) -> Result<Option<UserId>> {
        let cookie = self.get_private(USER_ID_COOKIE_NAME);
        Ok(cookie.map(|c| c.value().parse()).transpose()?.map(UserId))
    }

    fn set_user_id(&self, user_id: UserId) {
        self.add_private(
            CookieBuilder::new(USER_ID_COOKIE_NAME, user_id.0.to_string())
                .http_only(true)
                .secure(true)
                .permanent()
                .finish(),
        );
    }
}

const USER_ID_COOKIE_NAME: &str = "user-id";

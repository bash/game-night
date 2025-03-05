use anyhow::{Error, Result};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome};
use rocket::tokio::sync::Mutex;
use rocket::{async_trait, Phase, Request, Rocket};
use serde::Deserialize;

use crate::database::Repository;
use crate::RocketExt;

#[derive(Debug)]
pub(crate) struct UriBuilder<'a> {
    #[doc(hidden)]
    pub(crate) repository: Mutex<Box<dyn Repository>>,
    #[doc(hidden)]
    pub(crate) prefix: UrlPrefix<'a>,
}

impl UriBuilder<'_> {
    pub(crate) fn into_static(self) -> UriBuilder<'static> {
        UriBuilder {
            repository: self.repository,
            prefix: self.prefix.to_static(),
        }
    }
}

#[macro_export]
macro_rules! uri {
    (auto_login ($user:expr, $valid_until:expr); $builder: expr, $($t:tt)*) => {{
        async {
            let builder: &$crate::uri::UriBuilder = &($builder);
            let user: &$crate::users::User = $user;
            let token = $crate::login::LoginToken::generate_reusable(user.id, $valid_until, &mut ::rand::rng());
            match builder.repository.lock().await.add_login_token(&token).await {
                Ok(_) => Ok(::rocket::http::uri::Absolute::parse_owned($crate::login::with_autologin_token($crate::uri!($builder, $($t)*), &token)).unwrap()),
                Err(e) => Err(e),
            }
        }
    }};
    ($builder: expr, $($t:tt)*) => {{
        let builder: &$crate::uri::UriBuilder = &($builder);
        ::rocket::uri!(builder.prefix.0.clone(), $($t)*)
    }};
    ($($t:tt)*) => {{
        ::rocket::uri!($($t)*)
    }};
}

#[async_trait]
impl<'r> FromRequest<'r> for UriBuilder<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .rocket()
            .uri_builder()
            .await
            .or_error(Status::InternalServerError)
    }
}

pub(crate) trait HasUriBuilder {
    async fn uri_builder(&self) -> Result<UriBuilder<'_>>;
}

impl<P: Phase> HasUriBuilder for Rocket<P> {
    async fn uri_builder(&self) -> Result<UriBuilder<'_>> {
        Ok(UriBuilder {
            prefix: url_prefix(self)?,
            repository: Mutex::new(self.repository().await?),
        })
    }
}

#[doc(hidden)]
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct UrlPrefix<'a>(pub(crate) Absolute<'a>);

impl UrlPrefix<'_> {
    fn to_static(&self) -> UrlPrefix<'static> {
        UrlPrefix(Absolute::parse_owned(self.0.to_string()).unwrap())
    }
}

fn url_prefix<P: Phase>(rocket: &Rocket<P>) -> Result<UrlPrefix<'_>> {
    rocket
        .figment()
        .extract_inner("url_prefix")
        .map(UrlPrefix)
        .map_err(Into::into)
}

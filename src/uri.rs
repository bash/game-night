use crate::database::Repository;
use crate::impl_from_request_for_service;
use crate::services::{Resolve, ResolveContext};
use anyhow::Result;
use rocket::http::uri::Absolute;
use rocket::tokio::sync::Mutex;
use rocket::{Phase, Rocket};
use serde::Deserialize;

#[derive(Debug)]
pub(crate) struct UriBuilder {
    #[doc(hidden)]
    pub(crate) repository: Mutex<Box<dyn Repository>>,
    #[doc(hidden)]
    pub(crate) prefix: UrlPrefix<'static>,
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

impl Resolve for UriBuilder {
    async fn resolve(ctx: &ResolveContext<'_>) -> Result<Self> {
        Ok(UriBuilder {
            prefix: url_prefix(ctx.rocket())?.to_static(),
            repository: Mutex::new(ctx.resolve().await?),
        })
    }
}

impl_from_request_for_service!(UriBuilder);

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

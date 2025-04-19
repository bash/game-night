use anyhow::{Error, Result};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Orbit, Request, Rocket};
use std::future::Future;
use std::ops::{Deref, DerefMut};

pub(crate) trait Resolve: Sized {
    fn resolve(ctx: &ResolveContext<'_>) -> impl Future<Output = Result<Self>> + Send;
}

pub(crate) struct ResolveContext<'a>(&'a Rocket<Orbit>);

impl ResolveContext<'_> {
    pub(crate) fn rocket(&self) -> &Rocket<Orbit> {
        self.0
    }

    pub(crate) fn resolve<'a, T: Resolve + 'a>(
        &'a self,
    ) -> impl Future<Output = Result<T>> + Send + 'a {
        T::resolve(self)
    }
}

pub(crate) struct Service<R: Resolve>(pub(crate) R);

impl<R: Resolve> Service<R> {
    pub(crate) fn into_inner(self) -> R {
        self.0
    }
}

impl<R: Resolve> Deref for Service<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<R: Resolve> DerefMut for Service<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait]
impl<'r, R: Resolve> FromRequest<'r> for Service<R> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let context = ResolveContext(request.rocket());
        match R::resolve(&context).await {
            Ok(instance) => Outcome::Success(Service(instance)),
            Err(error) => Outcome::Error((Status::ServiceUnavailable, error)),
        }
    }
}

pub(crate) trait RocketResolveExt {
    fn resolve<'a, T: Resolve + 'a>(&'a self) -> impl Future<Output = Result<T>> + Send + 'a;
}

impl RocketResolveExt for Rocket<Orbit> {
    // Manual async fn because the compiler is not happy
    // if we omit the + 'a constraint.
    #[allow(clippy::manual_async_fn)]
    fn resolve<'a, T: Resolve + 'a>(&'a self) -> impl Future<Output = Result<T>> + Send + 'a {
        async {
            let ctx = ResolveContext(self);
            T::resolve(&ctx).await
        }
    }
}

#[macro_export]
macro_rules! impl_from_request_for_service {
    (<$($generic:ident $(: $constraint:ident)?),*> $R:ty) => {
        #[rocket::async_trait]
        impl<'r, $($generic $(: $constraint)?),*> rocket::request::FromRequest<'r> for $R {
            type Error = anyhow::Error;

            async fn from_request(
                request: &'r rocket::request::Request<'_>,
            ) -> rocket::request::Outcome<Self, Self::Error> {
                request
                    .guard::<$crate::services::Service<$R>>()
                    .await
                    .map(|s| s.into_inner())
            }
        }
    };
    ($R:ty) => {
        impl_from_request_for_service!(<> $R);
    };
}

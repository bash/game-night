use crate::users::User;
use anyhow::Error;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use std::marker::PhantomData;
use std::ops::Deref;

pub(crate) struct AuthorizedTo<P>(User, PhantomData<P>);

pub(crate) trait UserPredicate {
    fn is_satisfied(user: &User) -> bool;
}

#[async_trait]
impl<'r, P: UserPredicate> FromRequest<'r> for AuthorizedTo<P> {
    type Error = Option<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user: User = try_outcome!(request.guard().await);
        if let Some(result) = AuthorizedTo::new(user) {
            Outcome::Success(result)
        } else {
            Outcome::Failure((Status::Forbidden, None))
        }
    }
}

impl<P> AuthorizedTo<P>
where
    P: UserPredicate,
{
    pub(crate) fn new(inner: User) -> Option<Self> {
        P::is_satisfied(&inner).then_some(Self(inner, PhantomData))
    }
}

impl<P> Deref for AuthorizedTo<P> {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> AuthorizedTo<P> {
    fn into_inner(self) -> User {
        self.0
    }
}

pub(crate) struct Invite;

impl UserPredicate for Invite {
    fn is_satisfied(user: &User) -> bool {
        user.can_invite()
    }
}

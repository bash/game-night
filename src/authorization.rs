use crate::template::PageBuilder;
use crate::users::User;
use anyhow::Error;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, catch, catchers, Catcher, Request};
use rocket_dyn_templates::{context, Template};
use std::marker::PhantomData;
use std::ops::Deref;

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![forbidden]
}

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

pub(crate) struct ManagePoll;

impl UserPredicate for ManagePoll {
    fn is_satisfied(user: &User) -> bool {
        user.can_manage_poll()
    }
}

#[catch(403)]
async fn forbidden(request: &Request<'_>) -> Template {
    let page = PageBuilder::from_request(request)
        .await
        .expect("Page builder guard is infallible");
    let type_ = request.uri().try_into().unwrap_or_default();
    page.type_(type_).render("errors/403", context! {})
}

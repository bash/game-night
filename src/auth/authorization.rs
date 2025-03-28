use crate::event::StatefulEvent;
use crate::poll::Poll;
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::users::{Role, User};
use anyhow::{anyhow, Error};
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, catch, catchers, Catcher, Request};
use rocket_dyn_templates::{context, Template};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![forbidden]
}

pub(crate) struct AuthorizedTo<P>(User, PhantomData<P>);

pub(crate) trait UserPredicate {
    fn is_satisfied(user: &User) -> bool;
}

#[async_trait]
impl<'r, P: UserPredicate> FromRequest<'r> for AuthorizedTo<P> {
    type Error = Arc<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let user: User = try_outcome!(request.guard().await);
        if let Some(result) = AuthorizedTo::new(user) {
            Outcome::Success(result)
        } else {
            Outcome::Forward(Status::Forbidden)
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

pub(crate) struct ManageUsers;

impl UserPredicate for ManageUsers {
    fn is_satisfied(user: &User) -> bool {
        user.can_manage_users()
    }
}

pub(crate) fn is_invited(user: &User, event: &StatefulEvent) -> bool {
    let group = event.restrict_to();
    user.role == Role::Admin
        || group.is_none_or(|group| group.has_member(user))
        || event.has_organizer(user)
}

pub(crate) fn can_answer_strongly(user: &User, poll: &Poll) -> bool {
    user.can_answer_strongly() || poll.event.has_organizer(user)
}

#[catch(403)]
async fn forbidden(request: &Request<'_>) -> HttpResult<Template> {
    let page = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?;
    Ok(page.render("errors/403", context! {}))
}

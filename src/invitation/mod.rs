use crate::auth::{AuthorizedTo, Invite};
use crate::register::rocket_uri_macro_register_page;
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::models::UserV2;
use crate::users::{Role, User, UserId, UserQueries};
use itertools::Itertools as _;
use rand::rng;
use rocket::form::Form;
use rocket::http::uri::Absolute;
use rocket::{get, post, routes, FromForm, FromFormField, Route};
use std::num::NonZeroU32;
use time::{Duration, OffsetDateTime};

mod wordlist;
pub(crate) use self::wordlist::*;
mod passphrase;
pub(crate) use self::passphrase::*;
mod models;
pub(crate) use models::*;
mod queries;
pub(crate) use queries::*;
mod commands;
pub(crate) use commands::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![invite_page, generate_invitation]
}

#[get("/invite")]
async fn invite_page(
    user: AuthorizedTo<Invite>,
    mut users: UserQueries,
    page: PageContextBuilder<'_>,
) -> HttpResult<Templated<InvitePage>> {
    let users = users.active().await?;
    let user = user.into_inner();
    Ok(Templated(InvitePage {
        users,
        user,
        ctx: page.build(),
    }))
}

#[derive(Template, Debug)]
#[template(path = "invitation/invite.html")]
struct InvitePage {
    users: Vec<UserV2>,
    user: User,
    ctx: PageContext,
}

#[post("/invite", data = "<form>")]
async fn generate_invitation(
    _user: AuthorizedTo<Invite>,
    page: PageContextBuilder<'_>,
    mut invitations: InvitationCommands,
    form: Form<GenerateInvitationData>,
    uri_builder: UriBuilder,
) -> HttpResult<Templated<InvitationPage>> {
    let form = form.into_inner();
    let lifetime = Duration::days(i64::from(u32::from(form.lifetime_in_days)));
    let valid_until = OffsetDateTime::now_utc() + lifetime;
    let invitation = NewInvitation::builder()
        .role(Role::Guest)
        .created_by(form.inviter)
        .valid_until(valid_until)
        .comment(&form.comment)
        .build(&mut rng());
    let invitation = invitations.add(invitation).await?;
    let passphrase = invitation.passphrase.clone();
    let register_uri = uri!(
        uri_builder,
        register_page(passphrase = Some(invitation.passphrase))
    );
    Ok(Templated(InvitationPage {
        passphrase,
        register_uri,
        form,
        ctx: page.build(),
    }))
}

#[derive(Template, Debug)]
#[template(path = "invitation/invitation.html")]
struct InvitationPage {
    passphrase: Passphrase,
    register_uri: Absolute<'static>,
    form: GenerateInvitationData,
    ctx: PageContext,
}

#[derive(Debug, FromForm)]
struct GenerateInvitationData {
    lifetime_in_days: NonZeroU32,
    inviter: UserId,
    comment: String,
}

#[derive(Debug, Clone, Copy, FromFormField)]
enum InvitationLifetime {
    Short,
    Long,
}

impl From<InvitationLifetime> for Duration {
    fn from(value: InvitationLifetime) -> Self {
        use InvitationLifetime::*;
        match value {
            Short => Duration::days(30),
            Long => Duration::days(365),
        }
    }
}

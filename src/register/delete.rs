use crate::decorations::Random;
use crate::invitation::{InvitationCommands, NewInvitation, Passphrase};
use crate::login::{Logout, RedirectUri};
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::users::{User, UserCommands};
use rand::rng;
use rocket::response::Responder;
use rocket::{get, post, uri};
use time::{Duration, OffsetDateTime};

#[get("/profile/delete")]
pub(crate) fn delete_profile_page(page: PageContextBuilder, user: User) -> impl Responder {
    Templated(DeleteProfilePage {
        user,
        random: Random::default(),
        ctx: page.build(),
    })
}

#[post("/profile/delete")]
pub(crate) async fn delete_profile(
    mut invitations: InvitationCommands,
    mut users: UserCommands,
    user: User,
) -> HttpResult<Logout> {
    let invitation = invitations.add(goodbye_invitation(&user)).await?;
    users.remove(user.id).await?;
    let redirect_uri = RedirectUri(uri!(profile_deleted_page(user.name, invitation.passphrase)));
    Ok(Logout(redirect_uri))
}

fn goodbye_invitation(user: &User) -> NewInvitation {
    let valid_until = OffsetDateTime::now_utc() + Duration::days(365);
    NewInvitation::builder()
        .role(user.role)
        .valid_until(valid_until)
        .comment(format!("Goodbye invitation for '{}'", user.name))
        .build(&mut rng())
}

#[get("/profile/deleted?<name>&<passphrase>")]
pub(crate) fn profile_deleted_page(
    page: PageContextBuilder,
    name: String,
    passphrase: Passphrase,
) -> impl Responder {
    Templated(ProfileDeletedPage {
        name,
        passphrase,
        random: Random::default(),
        ctx: page.build(),
    })
}

#[derive(Template, Debug)]
#[template(path = "register/delete.html")]
pub(crate) struct DeleteProfilePage {
    pub(crate) user: User,
    pub(crate) random: Random,
    pub(crate) ctx: PageContext,
}

#[derive(Template, Debug)]
#[template(path = "register/deleted.html")]
pub(crate) struct ProfileDeletedPage {
    pub(crate) name: String,
    pub(crate) passphrase: Passphrase,
    pub(crate) random: Random,
    pub(crate) ctx: PageContext,
}

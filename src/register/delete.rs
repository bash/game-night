use crate::database::Repository;
use crate::invitation::{Invitation, Passphrase};
use crate::login::{Logout, RedirectUri};
use crate::template::PageBuilder;
use crate::users::User;
use anyhow::{Error, Result};
use rocket::response::Debug;
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};
use time::{Duration, OffsetDateTime};

#[get("/profile/delete")]
pub(crate) fn delete_profile_page(page: PageBuilder, _user: User) -> Template {
    page.render("register/delete", context! {})
}

#[post("/profile/delete")]
pub(crate) async fn delete_profile(
    mut repository: Box<dyn Repository>,
    user: User,
) -> Result<Logout, Debug<Error>> {
    let invitation = repository.add_invitation(goodbye_invitation(&user)).await?;
    repository.delete_user(user.id).await?;
    let redirect_uri = RedirectUri(uri!(profile_deleted_page(user.name, invitation.passphrase)));
    Ok(Logout(redirect_uri))
}

fn goodbye_invitation(user: &User) -> Invitation<()> {
    let valid_until = OffsetDateTime::now_utc() + Duration::days(365);
    Invitation::builder()
        .role(user.role)
        .valid_until(valid_until)
        .comment(format!("Goodbye invitation for '{}'", user.name))
        .build()
}

#[get("/profile/deleted?<name>&<passphrase>")]
pub(crate) fn profile_deleted_page(
    page: PageBuilder,
    name: String,
    passphrase: Passphrase,
) -> Template {
    page.render("register/deleted", context! { name, passphrase })
}

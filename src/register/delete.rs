use crate::database::Repository;
use crate::invitation::Passphrase;
use crate::login::{Logout, RedirectUri};
use crate::template::PageBuilder;
use crate::users::User;
use anyhow::Error;
use rocket::response::Debug;
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};

#[get("/profile/delete")]
pub(crate) fn delete_profile(page: PageBuilder, _user: User) -> Template {
    page.render("register/delete", context! {})
}

#[post("/profile/delete")]
pub(crate) fn delete_profile_page(
    _repository: Box<dyn Repository>,
    user: User,
) -> Result<Logout, Debug<Error>> {
    let invitation = Passphrase(vec![
        "foo".to_string(),
        "bar".to_string(),
        "baz".to_string(),
        "qux".to_string(),
    ]);
    let redirect_uri = RedirectUri(uri!(profile_deleted_page(user.name, invitation)));
    Ok(Logout(redirect_uri))
}

#[get("/profile/deleted?<name>&<passphrase>")]
pub(crate) fn profile_deleted_page(
    page: PageBuilder,
    name: String,
    passphrase: Passphrase,
) -> Template {
    page.render("register/deleted", context! { name, passphrase })
}

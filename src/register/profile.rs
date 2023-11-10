use crate::database::Repository;
use crate::push::WebPushKey;
use crate::template::PageBuilder;
use crate::users::rocket_uri_macro_list_users;
use crate::users::{User, UserPatch};
use anyhow::{Error, Result};
use base64::{engine::general_purpose, Engine as _};
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};

#[get("/profile")]
pub(crate) fn profile(page: PageBuilder, user: User, push_key: &WebPushKey) -> Template {
    let push_key = general_purpose::URL_SAFE_NO_PAD.encode(push_key.public_key());
    page.render(
        "register/profile",
        context! {
            can_update_name: user.can_update_name(),
            list_users_uri: user.can_manage_users().then(|| uri!(list_users())),
            push_key,
        },
    )
}

#[post("/profile", data = "<form>")]
pub(super) async fn update_profile(
    mut repository: Box<dyn Repository>,
    form: Form<UserPatch>,
    user: User,
) -> Result<Redirect, Debug<Error>> {
    let patch = filter_patch(&user, form.into_inner());
    repository.update_user(user.id, patch).await?;
    Ok(Redirect::to(uri!(profile)))
}

fn filter_patch(user: &User, mut patch: UserPatch) -> UserPatch {
    if !user.can_update_name() {
        patch.name = None;
    }

    patch
}

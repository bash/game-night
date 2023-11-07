use crate::database::Repository;
use crate::template::PageBuilder;
use crate::users::{User, UserPatch};
use anyhow::{Error, Result};
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};

#[get("/register/profile")]
pub(super) fn profile(page: PageBuilder, user: User) -> Template {
    page.render(
        "register/profile",
        context! { can_update_name: user.can_update_name() },
    )
}

#[post("/register/profile", data = "<form>")]
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

use crate::database::Repository;
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserPatch};
use anyhow::{Error, Result};
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};

#[get("/register/profile")]
pub(super) fn profile(page: PageBuilder, _guard: User) -> Template {
    page.type_(PageType::Register)
        .render("register/profile", context! {})
}

#[post("/register/profile", data = "<form>")]
pub(super) async fn update_profile(
    mut repository: Box<dyn Repository>,
    form: Form<UserPatch>,
    user: User,
) -> Result<Redirect, Debug<Error>> {
    repository.update_user(user.id, form.into_inner()).await?;
    Ok(Redirect::to(uri!(profile)))
}

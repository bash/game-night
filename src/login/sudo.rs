use super::RedirectUri;
use crate::auth::{AuthorizedTo, CookieJarExt, LoginState, ManageUsers};
use crate::users::{User, UserId};
use anyhow::Error;
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::{Debug, Redirect};
use rocket::{post, FromForm};
use LoginState::*;

#[post("/sudo/enter", data = "<form>")]
pub(super) fn enter(
    form: Form<SudoForm>,
    cookies: &'_ CookieJar<'_>,
    _guard: AuthorizedTo<ManageUsers>,
) -> Result<Redirect, Debug<Error>> {
    cookies.set_login_state(cookies.login_state()?.impersonate(UserId(form.user)));
    Ok(Redirect::to("/"))
}

#[post("/sudo/exit", data = "<form>")]
pub(super) fn exit(
    form: Form<ExitSudoForm>,
    cookies: &'_ CookieJar<'_>,
    _guard: User,
) -> Result<Redirect, Debug<Error>> {
    if let Impersonating { original, .. } = cookies.login_state()? {
        cookies.set_login_state(Authenticated(original));
    }

    Ok(Redirect::to(form.into_inner().redirect))
}

#[derive(Debug, FromForm)]
pub(super) struct SudoForm {
    user: i64,
}

#[derive(Debug, FromForm)]
pub(super) struct ExitSudoForm {
    redirect: RedirectUri,
}

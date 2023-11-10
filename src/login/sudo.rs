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
    if let Authenticated(user, original) = cookies.login_state()? {
        cookies.set_login_state(Authenticated(
            UserId(form.user),
            Some(original.unwrap_or(user)),
        ));
    }

    Ok(Redirect::to(form.into_inner().redirect))
}

#[post("/sudo/exit", data = "<form>")]
pub(super) fn exit(
    form: Form<ExitSudoForm>,
    cookies: &'_ CookieJar<'_>,
    _guard: User,
) -> Result<Redirect, Debug<Error>> {
    if let Authenticated(_, Some(original)) = cookies.login_state()? {
        cookies.set_login_state(Authenticated(original, None));
    }

    Ok(Redirect::to(form.into_inner().redirect))
}

#[derive(Debug, FromForm)]
pub(super) struct SudoForm {
    user: i64,
    redirect: RedirectUri,
}

#[derive(Debug, FromForm)]
pub(super) struct ExitSudoForm {
    redirect: RedirectUri,
}

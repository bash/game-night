use super::redirect_to;
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
    form: Form<SudoForm<'_>>,
    cookies: &'_ CookieJar<'_>,
    _guard: AuthorizedTo<ManageUsers>,
) -> Result<Redirect, Debug<Error>> {
    if let Authenticated(user, original) = cookies.login_state()? {
        cookies.set_login_state(Authenticated(
            UserId(form.user),
            Some(original.unwrap_or(user)),
        ));
    }

    Ok(redirect_to(form.redirect).unwrap_or_else(|| Redirect::to("/")))
}

#[post("/sudo/exit", data = "<form>")]
pub(super) fn exit(
    form: Form<ExitSudoForm<'_>>,
    cookies: &'_ CookieJar<'_>,
    _guard: User,
) -> Result<Redirect, Debug<Error>> {
    if let Authenticated(_, Some(original)) = cookies.login_state()? {
        cookies.set_login_state(Authenticated(original, None));
    }

    Ok(redirect_to(form.redirect).unwrap_or_else(|| Redirect::to("/")))
}

#[derive(Debug, FromForm)]
pub(super) struct SudoForm<'r> {
    user: i64,
    redirect: Option<&'r str>,
}

#[derive(Debug, FromForm)]
pub(super) struct ExitSudoForm<'r> {
    redirect: Option<&'r str>,
}

use super::{RedirectUri, RedirectUriExt as _};
use crate::auth::{CookieJarExt, LoginState};
use crate::database::Repository;
use crate::template::PageBuilder;
use anyhow::Error;
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, FromForm, Responder};
use rocket_dyn_templates::{context, Template};

#[get("/login/code?<redirect>")]
pub(super) async fn login_with_code_page<'r>(
    redirect: Option<RedirectUri>,
    page: PageBuilder<'r>,
) -> Template {
    page.uri(redirect).render("login_code", context! {})
}

#[post("/login/code?<redirect>", data = "<form>")]
pub(super) async fn login_with_code<'r>(
    page: PageBuilder<'r>,
    form: Form<LoginWithCodeData<'r>>,
    cookies: &'r CookieJar<'r>,
    mut repository: Box<dyn Repository>,
    redirect: Option<RedirectUri>,
) -> Result<LoginWithCodeResult, Debug<Error>> {
    use LoginWithCodeResult::*;
    if let Some(user_id) = repository.use_login_token(form.code).await? {
        cookies.set_login_state(LoginState::Authenticated(user_id, None));
        Ok(LoginWithCodeResult::success(Redirect::to(
            redirect.or_root(),
        )))
    } else {
        Ok(Error(
            page.render("login_code", context! { invalid_code: true }),
        ))
    }
}

#[derive(FromForm)]
pub(super) struct LoginWithCodeData<'r> {
    code: &'r str,
}

#[derive(Responder)]
pub(super) enum LoginWithCodeResult {
    Success(Box<Redirect>),
    Error(Template),
}

impl LoginWithCodeResult {
    fn success(redirect: Redirect) -> Self {
        LoginWithCodeResult::Success(Box::new(redirect))
    }
}

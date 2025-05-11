use super::{RedirectUri, RedirectUriExt as _};
use crate::auth::{CookieJarExt, LoginState};
use crate::database::Repository;
use crate::responder;
use crate::result::HttpResult;
use crate::template_v2::prelude::*;
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::{get, post, FromForm};

#[get("/login/code?<redirect>")]
pub(super) async fn login_with_code_page(
    redirect: Option<RedirectUri>,
    page: PageContextBuilder<'_>,
) -> Templated<LoginWithCodePage> {
    Templated(LoginWithCodePage {
        invalid_code: false,
        ctx: page.uri(redirect).build(),
    })
}

#[post("/login/code?<redirect>", data = "<form>")]
pub(super) async fn login_with_code<'r>(
    page: PageContextBuilder<'r>,
    form: Form<LoginWithCodeData<'r>>,
    cookies: &'r CookieJar<'r>,
    mut repository: Box<dyn Repository>,
    redirect: Option<RedirectUri>,
) -> HttpResult<LoginWithCodeResult> {
    if let Some(user_id) = repository.use_login_token(form.code).await? {
        cookies.set_login_state(LoginState::Authenticated(user_id));
        Ok(Redirect::to(redirect.or_root()).into())
    } else {
        Ok(Templated(LoginWithCodePage {
            invalid_code: true,
            ctx: page.uri(redirect).build(),
        })
        .into())
    }
}

#[derive(FromForm)]
pub(super) struct LoginWithCodeData<'r> {
    code: &'r str,
}

responder! {
    pub(super) enum LoginWithCodeResult {
        Success(Box<Redirect>),
        Error(Box<Templated<LoginWithCodePage>>),
    }
}

#[derive(Template, Debug)]
#[template(path = "login/code.html")]
pub(crate) struct LoginWithCodePage {
    invalid_code: bool,
    ctx: PageContext,
}

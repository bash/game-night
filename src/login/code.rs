use super::{page_type_from_redirect_uri, redirect_to};
use crate::auth::CookieJarExt;
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
    redirect: Option<&'r str>,
    page: PageBuilder<'r>,
) -> Template {
    page.type_(page_type_from_redirect_uri(redirect))
        .render("login_code", context! {})
}

#[post("/login/code?<redirect>", data = "<form>")]
pub(super) async fn login_with_code<'r>(
    page: PageBuilder<'r>,
    form: Form<LoginWithCodeData<'r>>,
    cookies: &'r CookieJar<'r>,
    mut repository: Box<dyn Repository>,
    redirect: Option<&'r str>,
) -> Result<LoginWithCodeResult, Debug<Error>> {
    use LoginWithCodeResult::*;
    if let Some(user_id) = repository.use_login_token(form.code).await? {
        cookies.set_user_id(user_id);
        Ok(LoginWithCodeResult::success(
            redirect_to(redirect).unwrap_or_else(|| Redirect::to("/")),
        ))
    } else {
        Ok(Error(
            page.type_(page_type_from_redirect_uri(redirect))
                .render("login_code", context! { invalid_code: true }),
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

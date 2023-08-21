use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::users::UserId;
use anyhow::Error;
use chrono::{DateTime, Duration, Local};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::{self, Debug, Redirect, Responder};
use rocket::{
    catch, catchers, get, post, routes, uri, Catcher, FromForm, Request, Response, Route,
};

pub(crate) fn routes() -> Vec<Route> {
    routes![login, login_page, login_with_token, logout]
}

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![redirect_to_login]
}

#[get("/login?<redirect>")]
async fn login_page<'r>(redirect: Option<&'r str>) {}

#[post("/login?<redirect>")]
async fn login<'r>(redirect: Option<&'r str>) -> Redirect {
    redirect_to(redirect)
}

#[get("/login-with?<token>&<redirect>")]
async fn login_with_token<'r>(
    token: &'r str,
    cookies: &'r CookieJar<'r>,
    mut repository: Box<dyn Repository>,
    redirect: Option<&'r str>,
) -> Result<Redirect, Debug<Error>> {
    if let Some(user_id) = repository.use_login_token(token).await? {
        cookies.set_user_id(user_id);
        Ok(redirect_to(redirect))
    } else {
        todo!()
    }
}

#[post("/logout", data = "<form>")]
async fn logout<'r>(cookies: &'r CookieJar<'r>, form: Form<LogoutData<'r>>) -> Logout<'r> {
    cookies.remove_user_id();
    Logout(form.redirect)
}

#[derive(FromForm)]
struct LogoutData<'r> {
    redirect: Option<&'r str>,
}

struct Logout<'r>(Option<&'r str>);

impl<'r> Responder<'r, 'static> for Logout<'r> {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(redirect_to(self.0).respond_to(request)?)
            .raw_header("Clear-Site-Data", "\"*\"")
            .ok()
    }
}

#[catch(401)]
async fn redirect_to_login(request: &Request<'_>) -> Redirect {
    let origin = request.uri().to_string();
    Redirect::to(uri!(login_page(redirect = Some(origin))))
}

fn redirect_to(redirect_url_from_query: Option<&str>) -> Redirect {
    redirect_url_from_query
        .and_then(|r| Origin::parse_owned(r.to_string()).ok().map(Redirect::to))
        .unwrap_or_else(|| Redirect::to("/"))
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct LoginToken {
    #[sqlx(rename = "type")]
    pub(crate) type_: LoginTokenType,
    pub(crate) token: String,
    pub(crate) user_id: UserId,
    pub(crate) valid_until: DateTime<Local>,
}

impl LoginToken {
    pub(crate) fn generate_one_time(user_id: UserId) -> Self {
        let one_time_token_expiration = Duration::minutes(30);
        let valid_until = Local::now() + one_time_token_expiration;
        Self {
            type_: LoginTokenType::OneTime,
            token: generate_token(),
            user_id,
            valid_until,
        }
    }

    pub(crate) fn generate_reusable(user_id: UserId, valid_until: DateTime<Local>) -> Self {
        Self {
            type_: LoginTokenType::Reusable,
            token: generate_token(),
            user_id,
            valid_until,
        }
    }
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub(crate) enum LoginTokenType {
    /// Short-lived one time tokens
    OneTime,
    /// Long-lived reusable tokens
    Reusable,
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(|d| d.to_string())
        .collect()
}

use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::email::EmailSender;
use crate::emails::LoginEmail;
use crate::template::PageBuilder;
use crate::users::UserId;
use crate::UrlPrefix;
use anyhow::Error;
use chrono::{DateTime, Duration, Local};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::{self, Debug, Redirect, Responder};
use rocket::{
    catch, catchers, get, post, routes, uri, Catcher, FromForm, Request, Response, Route, State,
};
use rocket_dyn_templates::{context, Template};

pub(crate) fn routes() -> Vec<Route> {
    routes![login, login_page, login_with, logout]
}

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![redirect_to_login]
}

#[get("/login?<redirect>")]
async fn login_page<'r>(redirect: Option<&'r str>, page: PageBuilder<'r>) -> Template {
    let page_type = redirect
        .and_then(|r| Origin::parse(r).ok())
        .and_then(|o| o.try_into().ok())
        .unwrap_or_default();
    page.type_(page_type)
        .render("login", context! { has_redirect: redirect.is_some() })
}

#[post("/login?<redirect>", data = "<form>")]
async fn login<'r>(
    mut repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    url_prefix: UrlPrefix<'r>,
    redirect: Option<&'r str>,
    form: Form<LoginData<'r>>,
) -> Result<Redirect, Debug<Error>> {
    if let Some(user) = repository.get_user_by_email(form.email).await? {
        let token = LoginToken::generate_one_time(user.id);
        let login_url = uri!(
            url_prefix.0,
            login_with(token = &token.token, redirect = redirect)
        );
        let email = LoginEmail {
            name: user.name.clone(),
            login_url: login_url.to_string(),
        };
        repository.add_login_token(token).await?;
        email_sender.send(user.mailbox()?, &email).await?;
    };
    todo!()
}

#[derive(FromForm)]
struct LoginData<'r> {
    email: &'r str,
}

#[get("/login-with?<token>&<redirect>")]
async fn login_with<'r>(
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
    match parse_redirect(redirect_url_from_query) {
        Some(redirect) => Redirect::to(redirect),
        None => Redirect::to("/"),
    }
}

fn parse_redirect<'a>(redirect: Option<&'a str>) -> Option<Origin<'static>> {
    redirect.and_then(|r| Origin::parse_owned(r.to_string()).ok())
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

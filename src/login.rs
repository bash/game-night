use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::email::EmailSender;
use crate::emails::LoginEmail;
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use crate::UrlPrefix;
use anyhow::{Error, Result};
use chrono::{DateTime, Duration, Local};
use lettre::message::Mailbox;
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

mod auto_login;
pub(crate) use auto_login::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        login,
        login_page,
        login_with,
        logout,
        auto_login::auto_login_redirect
    ]
}

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![redirect_to_login]
}

#[get("/login?<redirect>&<success>")]
async fn login_page<'r>(
    redirect: Option<&'r str>,
    success: Option<bool>,
    page: PageBuilder<'r>,
) -> Template {
    page.type_(page_type_from_redirect_uri(redirect)).render(
        "login",
        context! { has_redirect: redirect.is_some(), success },
    )
}

fn page_type_from_redirect_uri(redirect: Option<&str>) -> PageType {
    redirect
        .and_then(|r| Origin::parse(r).ok())
        .and_then(|o| o.try_into().ok())
        .unwrap_or_default()
}

#[post("/login?<redirect>", data = "<form>")]
async fn login<'r>(
    mut repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    url_prefix: UrlPrefix<'r>,
    redirect: Option<&'r str>,
    form: Form<LoginData<'r>>,
) -> Result<Redirect, Debug<Error>> {
    if let Some((mailbox, email)) =
        login_email_for(repository.as_mut(), url_prefix, &form.email, redirect).await?
    {
        email_sender.send(mailbox, &email).await?;
    }

    Ok(Redirect::to(uri!(login_page(
        success = Some(true),
        redirect = redirect
    ))))
}

async fn login_email_for(
    repository: &mut dyn Repository,
    url_prefix: UrlPrefix<'_>,
    email: &str,
    redirect: Option<&str>,
) -> Result<Option<(Mailbox, LoginEmail)>> {
    if repository.has_one_time_login_token(email).await? {
        Ok(None)
    } else if let Some(user) = repository.get_user_by_email(email).await? {
        generate_login_email(repository, url_prefix, redirect, user)
            .await
            .map(Some)
    } else {
        Ok(None)
    }
}

async fn generate_login_email(
    repository: &mut dyn Repository,
    url_prefix: UrlPrefix<'_>,
    redirect: Option<&str>,
    user: User,
) -> Result<(Mailbox, LoginEmail)> {
    let token = LoginToken::generate_one_time(user.id);
    repository.add_login_token(&token).await?;

    let login_url = uri!(
        url_prefix.0,
        login_with(token = &token.token, redirect = redirect)
    );
    let email = LoginEmail {
        name: user.name.clone(),
        login_url: login_url.to_string(),
    };

    Ok((user.mailbox()?, email))
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
        Ok(redirect_to_or_root(redirect))
    } else {
        Ok(redirect_to(redirect)
            .unwrap_or_else(|| Redirect::to(uri!(login_page(success = _, redirect = _)))))
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
    Redirect::to(uri!(login_page(redirect = Some(origin), success = _)))
}

fn redirect_to_or_root(redirect_url_from_query: Option<&str>) -> Redirect {
    redirect_to(redirect_url_from_query).unwrap_or_else(|| Redirect::to("/"))
}

fn redirect_to<'a>(redirect: Option<&'a str>) -> Option<Redirect> {
    redirect
        .and_then(|r| Origin::parse_owned(r.to_string()).ok())
        .map(|uri| Redirect::to(uri))
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

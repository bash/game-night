use crate::auth::{CookieJarExt, LoginState};
use crate::database::Repository;
use crate::email::{EmailMessage, EmailSender};
use crate::template::PageBuilder;
use crate::users::{User, UserId};
use anyhow::{Error, Result};
use lettre::message::Mailbox;
use rand::distributions::{Alphanumeric, Uniform};
use rand::Rng;
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::{self, Debug, Redirect, Responder};
use rocket::{
    catch, catchers, get, post, routes, uri, Catcher, FromForm, Request, Response, Route, State,
};
use rocket_dyn_templates::{context, Template};

mod auto_login;
pub(crate) use auto_login::*;
use serde::Serialize;
use time::{Duration, OffsetDateTime};
mod code;
mod keys;
pub(crate) use keys::*;
mod redirect;
pub(crate) use redirect::*;
mod sudo;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        login,
        login_page,
        code::login_with_code,
        code::login_with_code_page,
        logout,
        auto_login::auto_login_redirect,
        sudo::enter,
        sudo::exit
    ]
}

pub(crate) fn catchers() -> Vec<Catcher> {
    catchers![redirect_to_login]
}

#[get("/login?<redirect>")]
async fn login_page(
    redirect: Option<RedirectUri>,
    page: PageBuilder<'_>,
    user: Option<User>,
) -> LoginPage {
    match user {
        Some(_) => LoginPage::AlreadyLoggedIn(Redirect::to(redirect.or_root())),
        None => LoginPage::LoginRequired(
            page.uri(redirect.clone())
                .render("login", context! { has_redirect: redirect.is_some() }),
        ),
    }
}

#[derive(Debug, Responder)]
enum LoginPage {
    LoginRequired(Template),
    AlreadyLoggedIn(Redirect),
}

#[post("/login?<redirect>", data = "<form>")]
async fn login(
    builder: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    redirect: Option<RedirectUri>,
    form: Form<LoginData<'_>>,
) -> Result<Login, Debug<Error>> {
    if let Some((mailbox, email)) = login_email_for(repository.as_mut(), form.email).await? {
        email_sender.send(mailbox, &email).await?;
        Ok(Login::success(redirect))
    } else {
        Ok(Login::failure(builder, redirect, form.into_inner()))
    }
}

#[derive(Debug, Responder)]
enum Login {
    Success(Box<Redirect>),
    #[response(status = 400)]
    Failure(Template),
}

impl Login {
    fn success(redirect: Option<RedirectUri>) -> Login {
        Self::Success(Box::new(Redirect::to(uri!(code::login_with_code_page(
            redirect
        )))))
    }

    fn failure(builder: PageBuilder, redirect: Option<RedirectUri>, form: LoginData<'_>) -> Login {
        let context = context! {
            has_redirect: redirect.is_some(),
            form,
            error_message: "I don't know what to do with this email address, are you sure that you spelled it correctly? 🤔"
        };
        Self::Failure(builder.render("login", context))
    }
}

async fn login_email_for(
    repository: &mut dyn Repository,
    email: &str,
) -> Result<Option<(Mailbox, LoginEmail)>> {
    if let Some(user) = repository.get_user_by_email(email).await? {
        generate_login_email(repository, user).await.map(Some)
    } else {
        Ok(None)
    }
}

async fn generate_login_email(
    repository: &mut dyn Repository,
    user: User,
) -> Result<(Mailbox, LoginEmail)> {
    let token = LoginToken::generate_one_time(user.id);
    repository.add_login_token(&token).await?;

    let email = LoginEmail {
        name: user.name.clone(),
        code: token.token,
    };

    Ok((user.mailbox()?, email))
}

#[derive(Debug, FromForm, Serialize)]
struct LoginData<'r> {
    email: &'r str,
}

#[post("/logout", data = "<form>")]
async fn logout<'r>(cookies: &'r CookieJar<'r>, form: Form<LogoutData>) -> Logout {
    cookies.set_login_state(LoginState::Anonymous);
    Logout(form.into_inner().redirect)
}

#[derive(FromForm)]
struct LogoutData {
    redirect: RedirectUri,
}

struct Logout(RedirectUri);

impl<'r> Responder<'r, 'static> for Logout {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        Response::build_from(Redirect::to(self.0).respond_to(request)?)
            .raw_header("Clear-Site-Data", "\"*\"")
            .ok()
    }
}

#[catch(401)]
async fn redirect_to_login(request: &Request<'_>) -> Redirect {
    Redirect::to(uri!(login_page(redirect = Some(request.uri()))))
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct LoginToken {
    #[sqlx(rename = "type")]
    pub(crate) type_: LoginTokenType,
    pub(crate) token: String,
    pub(crate) user_id: UserId,
    pub(crate) valid_until: OffsetDateTime,
}

impl LoginToken {
    pub(crate) fn generate_one_time(user_id: UserId) -> Self {
        let one_time_token_expiration = Duration::minutes(10);
        let valid_until = OffsetDateTime::now_utc() + one_time_token_expiration;
        Self {
            type_: LoginTokenType::OneTime,
            token: generate_one_time_code(),
            user_id,
            valid_until,
        }
    }

    pub(crate) fn generate_reusable(user_id: UserId, valid_until: OffsetDateTime) -> Self {
        Self {
            type_: LoginTokenType::Reusable,
            token: generate_reusable_token(),
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

fn generate_reusable_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(|d| d.to_string())
        .collect()
}

fn generate_one_time_code() -> String {
    rand::thread_rng()
        .sample_iter(&Uniform::from(1..=9))
        .take(6)
        .map(|d| d.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize)]
struct LoginEmail {
    name: String,
    code: String,
}

impl EmailMessage for LoginEmail {
    fn subject(&self) -> String {
        "Let's Get You Logged In".to_owned()
    }

    fn template_name(&self) -> String {
        "login".to_owned()
    }
}

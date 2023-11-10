use crate::auth::{CookieJarExt, LoginState};
use crate::database::Repository;
use crate::email::EmailSender;
use crate::invitation::{Invitation, Passphrase};
use crate::template::PageBuilder;
use crate::users::{User, UserId};
use anyhow::{Error, Result};
use campaign::{Campaign, ProvidedCampaign};
use email_address::EmailAddress;
use lettre::message::Mailbox;
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::{Debug, Redirect};
use rocket::{get, post, routes, uri, Either, FromForm, Route, State};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::str::FromStr;
use verification::VerificationEmail;
use Either::*;
use StepResult::*;

mod campaign;
mod email_verification_code;
mod profile;
mod verification;
pub(crate) use email_verification_code::*;
pub(crate) use profile::*;

macro_rules! unwrap_or_return {
    ($result:expr, $e:ident => $ret:expr) => {
        match $result {
            Complete(x) => x,
            Pending($e) => return $ret,
        }
    };
}

macro_rules! pending {
    () => {
        Ok(Pending(None))
    };
    ($e:expr) => {
        Ok(Pending(Some($e.into())))
    };
}

pub(crate) fn routes() -> Vec<Route> {
    routes![
        getting_invited_redirect,
        getting_invited_page,
        register_redirect,
        register_page,
        register_form,
        profile::profile,
        profile::update_profile,
    ]
}

#[get("/getting-invited", rank = 10)]
async fn getting_invited_redirect(_user: User) -> Redirect {
    Redirect::to(Origin::ROOT)
}

#[get("/getting-invited", rank = 20)]
pub(crate) async fn getting_invited_page(page: PageBuilder<'_>) -> Template {
    page.render(
        "register/getting-invited",
        context! { register_uri: uri!(register_page(passphrase = Option::<Passphrase>::None)) },
    )
}

#[get("/register", rank = 10)]
async fn register_redirect(_user: User) -> Redirect {
    Redirect::to(uri!(profile::profile()))
}

#[get("/register?<passphrase>", rank = 20)]
async fn register_page(
    cookies: &CookieJar<'_>,
    repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    page: PageBuilder<'_>,
    campaign: Option<ProvidedCampaign<'_>>,
    passphrase: Option<Passphrase>,
) -> Result<Either<Template, Redirect>, Debug<Error>> {
    let form = RegisterForm::new_with_passphrase(passphrase);
    register(
        cookies,
        repository,
        email_sender.as_ref(),
        page,
        campaign,
        form,
        PassphraseSource::Query,
    )
    .await
}

#[post("/register", data = "<form>")]
async fn register_form(
    cookies: &CookieJar<'_>,
    repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    page: PageBuilder<'_>,
    campaign: Option<ProvidedCampaign<'_>>,
    form: Form<RegisterForm<'_>>,
) -> Result<Either<Template, Redirect>, Debug<Error>> {
    register(
        cookies,
        repository,
        email_sender.as_ref(),
        page,
        campaign,
        form.into_inner(),
        PassphraseSource::Form,
    )
    .await
}

async fn register(
    cookies: &CookieJar<'_>,
    mut repository: Box<dyn Repository>,
    email_sender: &dyn EmailSender,
    page: PageBuilder<'_>,
    campaign: Option<ProvidedCampaign<'_>>,
    form: RegisterForm<'_>,
    passphrase_source: PassphraseSource,
) -> Result<Either<Template, Redirect>, Debug<Error>> {
    let campaign = if let Some(campaign) = campaign {
        campaign
    } else {
        return Ok(Left(invalid_campaign(cookies, page)));
    };

    let invitation = unwrap_or_return!(
        invitation_code_step(&form, repository.as_mut()).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "invitation_code", error_message, form, campaign },
        )))
    );

    let user_details = unwrap_or_return!(
        user_details_step(&form, repository.as_mut(), email_sender).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "user_details", error_message, form, campaign, passphrase_source },
        )))
    );

    let user_id = unwrap_or_return!(
        email_verification_step(repository.as_mut(), &form, invitation, user_details, campaign.clone()).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "verify_email", error_message, form, campaign },
        )))
    );

    cookies.set_login_state(LoginState::Authenticated(user_id, None));
    Ok(Right(Redirect::to(uri!(crate::poll::poll_page()))))
}

fn invalid_campaign(cookies: &CookieJar<'_>, page: PageBuilder<'_>) -> Template {
    cookies.add(
        Cookie::build(("vary-smart", "A_cookie_for_very_smart_people"))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax),
    );
    page.render("register/invalid_campaign", context! {})
}

async fn invitation_code_step(
    form: &RegisterForm<'_>,
    repository: &mut dyn Repository,
) -> Result<StepResult<Invitation>> {
    let passphrase = if let Some(words) = &form.words {
        Passphrase::from_form_fields(words.iter().map(|w| w.as_str()))
    } else {
        return pending!();
    };

    let invitation = match repository.get_invitation_by_passphrase(&passphrase).await? {
        Some(invitation) => invitation,
        None => return pending!("That's not a valid invitation passphrase"),
    };

    if invitation.used_by.is_some() {
        return pending!("*Ruh-roh* Are you trying to use an invitation twice? Naughty! Naughty!");
    }

    Ok(Complete(invitation))
}

async fn user_details_step(
    form: &RegisterForm<'_>,
    repository: &mut dyn Repository,
    email_sender: &dyn EmailSender,
) -> Result<StepResult<UserDetails>> {
    let user_details = unwrap_or_return!(get_user_details_from_form(form)?, e => Ok(Pending(e)));

    let email_address = user_details.email_address.as_str();

    if repository.get_user_by_email(email_address).await?.is_some() {
        return pending!("You are already registered, you should try logging in instead :)");
    }

    if !repository.has_verification_code(email_address).await? {
        send_verification_email(repository, email_sender, &user_details).await?;
    }

    Ok(Complete(user_details))
}

fn get_user_details_from_form(form: &RegisterForm<'_>) -> Result<StepResult<UserDetails>> {
    let name = match form.name {
        Some(name) if !name.is_empty() => name,
        Some(_) => return pending!("Please enter your name"),
        None => return pending!(),
    };
    let email_address = match form.email_address.map(EmailAddress::from_str) {
        Some(Ok(addr)) => addr,
        Some(Err(_)) => return pending!("Please enter a valid email address"),
        None => return pending!(),
    };
    Ok(Complete(UserDetails {
        name: name.to_string(),
        email_address,
    }))
}

async fn send_verification_email(
    repository: &mut dyn Repository,
    email_sender: &dyn EmailSender,
    user_details: &UserDetails,
) -> Result<()> {
    let verification_code = EmailVerificationCode::generate(user_details.email_address.to_string());
    repository.add_verification_code(&verification_code).await?;

    let email = VerificationEmail {
        name: user_details.name.to_owned(),
        code: verification_code.code,
    };
    email_sender.send(user_details.clone().into(), &email).await
}

async fn email_verification_step(
    repository: &mut dyn Repository,
    form: &RegisterForm<'_>,
    invitation: Invitation,
    user_details: UserDetails,
    campaign: ProvidedCampaign<'_>,
) -> Result<StepResult<UserId>> {
    if let Pending(e) = use_verification_code(repository, form, &user_details).await? {
        return Ok(Pending(e));
    };

    let user_id = repository
        .add_user(
            invitation.clone(),
            new_user(invitation, user_details, campaign.into_inner()),
        )
        .await?;
    Ok(Complete(user_id))
}

async fn use_verification_code(
    repository: &mut dyn Repository,
    form: &RegisterForm<'_>,
    user_details: &UserDetails,
) -> Result<StepResult<()>> {
    let email = user_details.email_address.as_str();
    match form.email_verification_code {
        Some(code) if repository.use_verification_code(code, email).await? => Ok(Complete(())),
        Some(_) => pending!("That's not the correct code, maybe it has expired?"),
        None => pending!(),
    }
}

fn new_user(
    invitation: Invitation,
    user_details: UserDetails,
    campaign: Option<Campaign<'_>>,
) -> User<()> {
    invitation.to_user(
        user_details.name,
        user_details.email_address.to_string(),
        campaign.map(|c| c.name.to_owned()),
    )
}

enum StepResult<T> {
    Pending(Option<String>),
    Complete(T),
}

#[derive(FromForm, Serialize)]
pub(crate) struct RegisterForm<'r> {
    words: Option<Vec<String>>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<&'r str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum PassphraseSource {
    Query,
    Form,
}

impl<'a> RegisterForm<'a> {
    fn new_with_passphrase(passphrase: Option<Passphrase>) -> Self {
        Self {
            email_address: None,
            words: passphrase.map(|p| p.0),
            name: None,
            email_verification_code: None,
        }
    }
}

#[derive(Debug, Clone)]
struct UserDetails {
    email_address: EmailAddress,
    name: String,
}

impl From<UserDetails> for Mailbox {
    fn from(value: UserDetails) -> Self {
        Mailbox::new(
            Some(value.name),
            value.email_address.as_str().parse().unwrap(),
        )
    }
}

use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::email::EmailSender;
use crate::email_verification_code::EmailVerificationCode;
use crate::emails::VerificationEmail;
use crate::invitation::{Invitation, Passphrase};
use crate::template::{PageBuilder, PageType};
use crate::users::{User, UserId};
use anyhow::{Error, Result};
use campaign::Campaign;
use email_address::EmailAddress;
use lettre::message::Mailbox;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::{Debug, Redirect};
use rocket::{get, post, routes, Either, FromForm, Route, State};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::str::FromStr;
use Either::*;
use StepResult::*;

mod campaign;

macro_rules! unwrap_or_return {
    ($result:expr, $e:ident => $ret:expr) => {
        match $result {
            Complete(x) => x,
            Pending($e) => return $ret,
        }
    };
}

pub(crate) fn routes() -> Vec<Route> {
    routes![register_page, register]
}

#[get("/register")]
fn register_page(
    cookies: &CookieJar<'_>,
    page: PageBuilder<'_>,
    campaign: Option<Campaign<'_>>,
) -> Template {
    if campaign.is_none() {
        cookies.add(
            Cookie::build("vary-smart", "A_cookie_for_very_smart_people")
                .http_only(true)
                .secure(true)
                .same_site(SameSite::Strict)
                .finish(),
        );
    }

    page.type_(PageType::Register).render(
        "register",
        context! { step: "invitation_code", form: context! {}, is_campaign_valid: campaign.is_some(), campaign },
    )
}

#[post("/register", data = "<form>")]
async fn register(
    cookies: &CookieJar<'_>,
    form: Form<RegisterForm<'_>>,
    mut repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    page: PageBuilder<'_>,
    campaign: Campaign<'_>,
) -> Result<Either<Template, Redirect>, Debug<Error>> {
    let form = form.into_inner();
    let page = page.type_(PageType::Register);

    let invitation = unwrap_or_return!(
        invitation_code_step(&form, repository.as_mut()).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "invitation_code", error_message, form, is_campaign_valid: true, campaign },
        )))
    );

    let user_details = unwrap_or_return!(
        user_details_step(&form, repository.as_mut(), email_sender.as_ref()).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "user_details", error_message, form },
        )))
    );

    let user_id = unwrap_or_return!(
        email_verification_step(repository.as_mut(), &form, invitation, user_details, campaign).await?,
        error_message => Ok(Left(page.render(
            "register",
            context! { step: "verify_email", error_message, form },
        )))
    );

    cookies.set_user_id(user_id);
    Ok(Right(Redirect::to("/poll")))
}

async fn invitation_code_step(
    form: &RegisterForm<'_>,
    repository: &mut dyn Repository,
) -> Result<StepResult<Invitation>> {
    let passphrase = Passphrase(form.words.clone());
    Ok(repository
        .get_invitation_by_passphrase(&passphrase)
        .await?
        .map(Complete)
        .unwrap_or_else(|| Pending(Some("That's not a valid invitation passphrase".into()))))
}

async fn user_details_step(
    form: &RegisterForm<'_>,
    repository: &mut dyn Repository,
    email_sender: &dyn EmailSender,
) -> Result<StepResult<UserDetails>> {
    let user_details = unwrap_or_return!(get_user_details_from_form(form)?, e => Ok(Pending(e)));

    let email_address = user_details.email_address.as_str();
    if !repository.has_verification_code(email_address).await? {
        send_verification_email(repository, email_sender, &user_details).await?;
    }

    Ok(Complete(user_details))
}

fn get_user_details_from_form(form: &RegisterForm<'_>) -> Result<StepResult<UserDetails>> {
    let name = match form.name {
        Some(name) if !name.is_empty() => name,
        Some(_) => return Ok(Pending(Some("Please enter your name".into()))),
        None => return Ok(Pending(None)),
    };
    let email_address = match form.email_address.map(EmailAddress::from_str) {
        Some(Ok(addr)) => addr,
        Some(Err(_)) => return Ok(Pending(Some("Please enter a valid email address".into()))),
        None => return Ok(Pending(None)),
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
    campaign: Campaign<'_>,
) -> Result<StepResult<UserId>> {
    if let Pending(e) = use_verification_code(repository, form, &user_details).await? {
        return Ok(Pending(e));
    };

    let user_id = repository
        .add_user(
            invitation.clone(),
            new_user(invitation, user_details, campaign),
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
        Some(_) => Ok(Pending(Some(
            "That's not the correct code, maybe it has expired?".into(),
        ))),
        None => Ok(Pending(None)),
    }
}

fn new_user(invitation: Invitation, user_details: UserDetails, campaign: Campaign<'_>) -> User<()> {
    invitation.to_user(
        user_details.name,
        user_details.email_address.to_string(),
        campaign.into_inner().map(|c| c.to_owned()),
    )
}

enum StepResult<T> {
    Pending(Option<String>),
    Complete(T),
}

#[derive(FromForm, Serialize)]
pub(crate) struct RegisterForm<'r> {
    words: Vec<String>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<&'r str>,
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

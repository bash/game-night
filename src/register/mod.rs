use crate::auth::{CookieJarExt, LoginState};
use crate::database::Repository;
use crate::default;
use crate::email::{EmailSender, EmailTemplateContext};
use crate::invitation::{Invitation, Passphrase};
use crate::template::prelude::*;
use crate::users::{AstronomicalSymbol, User, UserCommands, UserId, UserQueries};
use anyhow::Result;
use campaign::{Campaign, ProvidedCampaign};
use email_address::EmailAddress;
use lettre::message::Mailbox;
use rand::{rng, Rng};
use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::{Redirect, Responder};
use rocket::{get, post, routes, uri, FromForm, Route, State};
use std::str::FromStr;
use verification::VerificationEmail;
use StepResult::*;

mod campaign;
mod delete;
mod email_verification_code;
mod profile;
mod verification;
use crate::decorations::Random;
use crate::result::HttpResult;
use crate::users::models::NewUser;
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
        delete::delete_profile_page,
        delete::delete_profile,
        delete::profile_deleted_page,
    ]
}

#[get("/getting-invited", rank = 10)]
async fn getting_invited_redirect(_user: User) -> Redirect {
    Redirect::to("/")
}

#[get("/getting-invited", rank = 20)]
pub(crate) async fn getting_invited_page(page: PageContextBuilder<'_>) -> impl Responder {
    Templated(GettingInvitedPage {
        register_uri: uri!(register_page(passphrase = Option::<Passphrase>::None)),
        ctx: page.build(),
    })
}

#[get("/register", rank = 10)]
async fn register_redirect(_user: User) -> Redirect {
    Redirect::to(uri!(profile::profile()))
}

#[allow(clippy::too_many_arguments)]
#[get("/register?<passphrase>", rank = 20)]
async fn register_page<'r>(
    cookies: &CookieJar<'_>,
    users: UserCommands,
    mut users_q: UserQueries,
    repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    page: PageContextBuilder<'_>,
    email_ctx: EmailTemplateContext,
    campaign: Option<ProvidedCampaign<'r>>,
    passphrase: Option<Passphrase>,
) -> HttpResult<RegisterResponse<'r>> {
    let form = RegisterForm::new_with_passphrase(passphrase);
    register(
        cookies,
        users,
        &mut users_q,
        repository,
        email_sender.as_ref(),
        page,
        email_ctx,
        campaign,
        form,
        PassphraseSource::Query,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
#[post("/register", data = "<form>")]
async fn register_form<'r>(
    cookies: &CookieJar<'_>,
    users: UserCommands,
    mut users_q: UserQueries,
    repository: Box<dyn Repository>,
    email_sender: &State<Box<dyn EmailSender>>,
    page: PageContextBuilder<'_>,
    email_ctx: EmailTemplateContext,
    campaign: Option<ProvidedCampaign<'r>>,
    form: Form<RegisterForm<'r>>,
) -> HttpResult<RegisterResponse<'r>> {
    register(
        cookies,
        users,
        &mut users_q,
        repository,
        email_sender.as_ref(),
        page,
        email_ctx,
        campaign,
        form.into_inner(),
        PassphraseSource::Form,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn register<'r>(
    cookies: &CookieJar<'_>,
    users: UserCommands,
    users_q: &mut UserQueries,
    mut repository: Box<dyn Repository>,
    email_sender: &dyn EmailSender,
    page: PageContextBuilder<'_>,
    email_ctx: EmailTemplateContext,
    campaign: Option<ProvidedCampaign<'r>>,
    form: RegisterForm<'r>,
    passphrase_source: PassphraseSource,
) -> HttpResult<RegisterResponse<'r>> {
    let campaign = if let Some(campaign) = campaign {
        campaign
    } else {
        return Ok(RegisterResponse::InvalidCampaign(Box::new(
            invalid_campaign(cookies, page),
        )));
    };

    let invitation = unwrap_or_return!(
        invitation_code_step(&form, repository.as_mut()).await?,
        error_message => Ok(RegisterPage {
            step: RegisterStep::InvitationCode,
            error_message,
            form,
            campaign: campaign.into_inner(),
            passphrase_source,
            ctx: page.build(),
        }.into())
    );

    let user_details = unwrap_or_return!(
        user_details_step(&form, repository.as_mut(), users_q, email_sender, email_ctx).await?,
        error_message => Ok(RegisterPage {
            step: RegisterStep::UserDetails,
            error_message,
            form,
            campaign: campaign.into_inner(),
            passphrase_source,
            ctx: page.build(),
        }.into())
    );

    let email_address = user_details.email_address.clone();
    let user_id = unwrap_or_return!(
        email_verification_step(repository.as_mut(), users, &form, invitation, user_details, campaign.clone()).await?,
        error_message => Ok(RegisterPage {
            step: RegisterStep::VerifyEmail(email_address),
            error_message,
            form,
            campaign: campaign.into_inner(),
            passphrase_source,
            ctx: page.build(),
        }.into())
    );

    cookies.set_login_state(LoginState::Authenticated(user_id));
    Ok(RegisterResponse::Redirect(Box::new(Redirect::to(uri!(
        crate::home::home_page()
    )))))
}

#[derive(Debug, Responder)]
pub(crate) enum RegisterResponse<'r> {
    Redirect(Box<Redirect>),
    Form(Box<Templated<RegisterPage<'r>>>),
    InvalidCampaign(Box<Templated<InvalidCampaignPage>>),
}

impl<'r> From<RegisterPage<'r>> for RegisterResponse<'r> {
    fn from(value: RegisterPage<'r>) -> Self {
        RegisterResponse::Form(Box::new(Templated(value)))
    }
}

fn invalid_campaign(
    cookies: &CookieJar<'_>,
    page: PageContextBuilder<'_>,
) -> Templated<InvalidCampaignPage> {
    cookies.add(
        Cookie::build(("vary-smart", "A_cookie_for_very_smart_people"))
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax),
    );
    Templated(InvalidCampaignPage { ctx: page.build() })
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
    users: &mut UserQueries,
    email_sender: &dyn EmailSender,
    email_ctx: EmailTemplateContext,
) -> Result<StepResult<UserDetails>> {
    let user_details = unwrap_or_return!(get_user_details_from_form(form)?, e => Ok(Pending(e)));

    let email_address = user_details.email_address.as_str();

    if users.by_email(email_address).await?.is_some() {
        return pending!("You are already registered, you should try logging in instead :)");
    }

    if !repository.has_verification_code(email_address).await? {
        send_verification_email(repository, email_sender, email_ctx, &user_details).await?;
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
    email_ctx: EmailTemplateContext,
    user_details: &UserDetails,
) -> Result<()> {
    let email_address = user_details.email_address.to_string();
    let code = EmailVerificationCode::generate(email_address, &mut rng());
    repository.add_verification_code(&code).await?;

    let email = VerificationEmail {
        name: user_details.name.to_owned(),
        code: code.code,
        random: Random::default(),
        ctx: email_ctx,
    };
    email_sender
        .send(user_details.clone().into(), &email, default())
        .await
}

async fn email_verification_step(
    repository: &mut dyn Repository,
    mut users: UserCommands,
    form: &RegisterForm<'_>,
    invitation: Invitation,
    user_details: UserDetails,
    campaign: ProvidedCampaign<'_>,
) -> Result<StepResult<UserId>> {
    if let Pending(e) = use_verification_code(repository, form, &user_details).await? {
        return Ok(Pending(e));
    };

    let new_user = new_user(&invitation, user_details, campaign.into_inner());
    let user = users.add(new_user, &invitation).await?;
    Ok(Complete(user.id))
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
    invitation: &Invitation,
    user_details: UserDetails,
    campaign: Option<Campaign<'_>>,
) -> NewUser {
    invitation.to_user(
        user_details.name,
        rng().random::<AstronomicalSymbol>(),
        user_details.email_address.to_string(),
        campaign.map(|c| c.name.to_owned()),
    )
}

enum StepResult<T> {
    Pending(Option<String>),
    Complete(T),
}

#[derive(FromForm, Debug)]
pub(crate) struct RegisterForm<'r> {
    words: Option<Vec<String>>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<&'r str>,
}

#[derive(Debug)]
enum PassphraseSource {
    Query,
    Form,
}

impl RegisterForm<'_> {
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

#[derive(Template, Debug)]
#[template(path = "register/getting-invited.html")]
pub(crate) struct GettingInvitedPage {
    register_uri: Origin<'static>,
    ctx: PageContext,
}

#[derive(Template, Debug)]
#[template(path = "register/invalid-campaign.html")]
pub(crate) struct InvalidCampaignPage {
    ctx: PageContext,
}

#[derive(Template, Debug)]
#[template(path = "register/register.html")]
pub(crate) struct RegisterPage<'r> {
    step: RegisterStep,
    error_message: Option<String>,
    form: RegisterForm<'r>,
    campaign: Option<Campaign<'r>>,
    passphrase_source: PassphraseSource,
    ctx: PageContext,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum RegisterStep {
    InvitationCode,
    UserDetails,
    VerifyEmail(EmailAddress),
}

impl RegisterPage<'_> {
    fn nth_word_or_empty(&self, n: usize) -> &str {
        self.form
            .words
            .as_ref()
            .and_then(|w| w.get(n))
            .map(|w| w.as_str())
            .unwrap_or_default()
    }
}

use crate::database::{Repository, SqliteRepository};
use crate::email::EmailSender;
use crate::email_verification_code::EmailVerificationCode;
use crate::emails::VerificationEmail;
use crate::invitation::{Invitation, Passphrase};
use crate::users::{User, UserId};
use crate::GameNightDatabase;
use anyhow::Result;
use email_address::EmailAddress;
use lettre::message::Mailbox;
use rocket::form::Form;
use rocket::{post, FromForm, State};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::str::FromStr;
use StepResult::*;

#[post("/register", data = "<form>")]
pub(crate) async fn register(
    form: Form<RegisterForm<'_>>,
    database: Connection<GameNightDatabase>,
    email_sender: &State<Box<dyn EmailSender>>,
) -> Template {
    let form = form.into_inner();
    let mut repository = SqliteRepository(database.into_inner());

    let invitation = match invitation_code_step(&form, &mut repository).await.unwrap() {
        Complete(i) => i,
        Pending(error_message) => {
            return Template::render(
                "register",
                context! { active_page: "register", step: "invitation_code", error_message, form },
            )
        }
    };

    let user_details = match user_details_step(&form, &mut repository, email_sender.as_ref())
        .await
        .unwrap()
    {
        Complete(d) => d,
        Pending(error_message) => {
            return Template::render(
                "register",
                context! { active_page: "register", step: "user_details", error_message, form },
            )
        }
    };

    let _user_id = match email_verification_step(&mut repository, &form, invitation, user_details)
        .await
        .unwrap()
    {
        Complete(id) => id,
        Pending(error_message) => {
            return Template::render(
                "register",
                context! { active_page: "register", step: "verify_email", error_message, form },
            );
        }
    };

    // TOOD: log user in, redirect
    Template::render("register", context! { active_page: "register", step: "" })
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
    let name = match form.name {
        Some(name) if name.len() >= 1 => name,
        Some(_) => return Ok(Pending(Some("Please enter your name".into()))),
        None => return Ok(Pending(None)),
    };
    let email_address = match form.email_address.map(EmailAddress::from_str) {
        Some(Ok(addr)) => addr,
        Some(Err(_)) => return Ok(Pending(Some("Please enter a valid email address".into()))),
        None => return Ok(Pending(None)),
    };

    if !repository
        .has_email_verification_code_for_email(email_address.as_str())
        .await?
    {
        let verification_code = EmailVerificationCode::generate(email_address.to_string());
        repository
            .add_email_verification_code(&verification_code)
            .await?;
        let email = VerificationEmail {
            name: name.to_owned(),
            code: verification_code.code,
        };
        email_sender
            .send(
                Mailbox::new(
                    Some(name.to_owned()),
                    email_address.as_str().parse().unwrap(),
                ),
                &email,
            )
            .await?;
    }

    Ok(Complete(UserDetails {
        name: name.to_string(),
        email_address,
    }))
}

async fn email_verification_step(
    repository: &mut dyn Repository,
    form: &RegisterForm<'_>,
    invitation: Invitation,
    user_details: UserDetails,
) -> Result<StepResult<UserId>> {
    if let Pending(e) = use_verification_code(repository, form, &user_details).await? {
        return Ok(Pending(e));
    };

    let user_id = repository
        .add_user(invitation.clone(), new_user(invitation, user_details))
        .await?;
    Ok(Complete(user_id))
}

async fn use_verification_code(
    repository: &mut dyn Repository,
    form: &RegisterForm<'_>,
    user_details: &UserDetails,
) -> Result<StepResult<()>> {
    let email = &user_details.email_address.as_str();
    match form.email_verification_code {
        Some(code) if repository.use_email_verification_code(code, email).await? => {
            Ok(Complete(()))
        }
        Some(_) => Ok(Pending(Some("That's not the correct code :/".into()))),
        None => Ok(Pending(None)),
    }
}

fn new_user(invitation: Invitation, user_details: UserDetails) -> User<()> {
    invitation.to_user(user_details.name, user_details.email_address.to_string())
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

struct UserDetails {
    email_address: EmailAddress,
    name: String,
}

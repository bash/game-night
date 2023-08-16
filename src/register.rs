use crate::database::{Repository, SqliteRepository};
use crate::invitation::{Invitation, Passphrase};
use crate::GameNightDatabase;
use anyhow::Error;
use email_address::EmailAddress;
use rocket::form::Form;
use rocket::{post, FromForm};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use std::borrow::Cow;

#[post("/register", data = "<form>")]
pub(crate) async fn register(
    form: Form<RegisterForm<'_>>,
    database: Connection<GameNightDatabase>,
) -> Template {
    let mut repository = SqliteRepository(database.into_inner());
    match to_register_step(form.into_inner(), &mut repository).await {
        Ok(_) => todo!(),
        Err(RegisterError::Validation(error_message)) => Template::render(
            "register",
            context! { active_page: "register", error_message },
        ),
        Err(RegisterError::Internal(_)) => todo!(),
    }
}

async fn to_register_step(
    form: RegisterForm<'_>,
    repository: &mut (dyn Repository + Send),
) -> Result<RegisterStep, RegisterError> {
    let passphrase = Passphrase(form.words);
    let invitation = repository
        .get_invitation_by_passphrase(&passphrase)
        .await?
        .ok_or(RegisterError::Validation(
            "That's not a valid invitation passphrase".into(),
        ))?;
    Ok(RegisterStep::InvitationCode { invitation })
}

pub(crate) enum RegisterError {
    Validation(Cow<'static, str>),
    Internal(Error),
}

impl From<Error> for RegisterError {
    fn from(value: Error) -> Self {
        RegisterError::Internal(value)
    }
}

#[derive(FromForm)]
pub(crate) struct RegisterForm<'r> {
    words: Vec<String>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<u64>,
}

#[derive(Debug)]
pub(crate) enum RegisterStep {
    InvitationCode {
        invitation: Invitation,
    },
    UserDetails {
        invitation: Invitation,
        email_address: EmailAddress,
        name: String,
    },
    EmailVerification {
        invitation: Invitation,
        email_address: EmailAddress,
        name: String,
        email_verification_code: u64,
    },
}

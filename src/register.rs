use crate::database::{Repository, SqliteRepository};
use crate::invitation::{Invitation, Passphrase};
use crate::GameNightDatabase;
use email_address::EmailAddress;
use rocket::form::Form;
use rocket::{post, FromForm};
use rocket_db_pools::Connection;
use std::convert::identity;

#[post("/register", data = "<form>")]
pub(crate) async fn register(
    form: Form<RegisterForm<'_>>,
    database: Connection<GameNightDatabase>,
) -> String {
    to_register_step(
        form.into_inner(),
        &mut SqliteRepository(database.into_inner()),
    )
    .await
    .map(|step| format!("{:#?}", step))
    .unwrap_or_else(identity)
}

async fn to_register_step(
    form: RegisterForm<'_>,
    repository: &mut (dyn Repository + Send),
) -> Result<RegisterStep, String> {
    let passphrase = Passphrase(form.words);
    let invitation = repository
        .get_invitation_by_passphrase(&passphrase)
        .await
        .unwrap()
        .ok_or("Invitation not found")?;
    Ok(RegisterStep::InvitationCode { invitation })
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

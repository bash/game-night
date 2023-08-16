use crate::database::{Repository, SqliteRepository};
use crate::invitation::{Invitation, Passphrase};
use crate::GameNightDatabase;
use anyhow::Result;
use email_address::EmailAddress;
use rocket::form::Form;
use rocket::{post, FromForm};
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use StepResult::*;

#[post("/register", data = "<form>")]
pub(crate) async fn register(
    form: Form<RegisterForm<'_>>,
    database: Connection<GameNightDatabase>,
) -> Template {
    let form = form.into_inner();
    let mut repository = SqliteRepository(database.into_inner());

    let _invitation = match invitation_code_step(&form, &mut repository).await.unwrap() {
        Complete(i) => i,
        Error(error_message) => {
            return Template::render(
                "register",
                context! { active_page: "register", step: "invitation_code", error_message, form },
            )
        }
    };

    Template::render(
        "register",
        context! { active_page: "register", step: "user_details", form },
    )
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
        .unwrap_or_else(|| Error("That's not a valid invitation passphrase".into())))
}

enum StepResult<T> {
    Error(String),
    Complete(T),
}

#[derive(FromForm, Serialize)]
pub(crate) struct RegisterForm<'r> {
    words: Vec<String>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<u64>,
}

struct UserDetails {
    email_address: EmailAddress,
    name: String,
}

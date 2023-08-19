use crate::users::User;
use anyhow::{Context, Result};
use database::SqliteRepository;
use diceware_wordlists::EFF_LONG_WORDLIST;
use email::{EmailSender, EmailSenderImpl};
use keys::GameNightKeys;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::fs::FileServer;
use rocket::serde::json::Json;
use rocket::{error, get, launch, routes, Build, Config, Rocket};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};

mod authentication;
mod database;
mod email;
mod email_verification_code;
mod emails;
mod invitation;
mod keys;
mod register;
mod users;

#[launch]
fn rocket() -> _ {
    rocket::custom(figment())
        .mount(
            "/",
            routes![
                get_index_page,
                get_invite_page,
                get_register_page,
                get_poll_page,
                get_eff_long_wordlist,
                register::register
            ],
        )
        .mount("/", FileServer::from("public"))
        .attach(Template::fairing())
        .attach(GameNightDatabase::init())
        .attach(initialize_email_sender())
        .attach(invite_admin_user())
}

fn figment() -> Figment {
    let keys = GameNightKeys::read_or_generate().unwrap();
    Config::figment().merge(("secret_key", &keys.rocket_secret_key))
}

#[get("/")]
fn get_index_page() -> Template {
    Template::render("index", context! { active_page: "home" })
}

#[get("/invite")]
fn get_invite_page(user: Option<User>) -> Template {
    Template::render("invite", context! { active_page: "invite", user })
}

#[get("/register")]
fn get_register_page(user: Option<User>) -> Template {
    Template::render(
        "register",
        context! { active_page: "register", step: "invitation_code", user, form: context! {} },
    )
}

#[get("/poll")]
fn get_poll_page(user: User) -> Template {
    Template::render("poll", context! { active_page: "poll", user })
}

#[get("/_api/eff-long-wordlist")]
fn get_eff_long_wordlist() -> Json<Vec<&'static str>> {
    Json(EFF_LONG_WORDLIST.into_iter().collect())
}

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

fn invite_admin_user() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Invite Admin User", |rocket| {
        Box::pin(async {
            match try_invite_admin_user(&rocket).await {
                Ok(_) => Ok(rocket),
                Err(e) => {
                    error!("{}", e);
                    Err(rocket)
                }
            }
        })
    })
}

async fn try_invite_admin_user(rocket: &Rocket<Build>) -> Result<()> {
    let connection = GameNightDatabase::fetch(rocket)
        .context("database not configured")?
        .get()
        .await
        .context("failed to retrieve database")?;
    invitation::invite_admin_user(&mut SqliteRepository(connection))
        .await
        .context("failed to invite admin user")
}

fn initialize_email_sender() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Email Sender", |rocket| {
        Box::pin(async {
            match EmailSenderImpl::from_figment(rocket.figment()).await {
                Ok(sender) => Ok(rocket.manage(Box::new(sender) as Box<dyn EmailSender>)),
                Err(error) => {
                    error!("failed to initialize email sender: {}", error);
                    Err(rocket)
                }
            }
        })
    })
}

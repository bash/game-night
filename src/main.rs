use anyhow::{Context, Result};
use database::SqliteRepository;
use email::{EmailSender, EmailSenderImpl};
use keys::GameNightKeys;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::fs::FileServer;
use rocket::{error, Build, Config, Rocket};
use rocket::{get, launch, routes};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};

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
fn get_invite_page() -> Template {
    Template::render("invite", context! { active_page: "invite" })
}

#[get("/register")]
fn get_register_page() -> Template {
    Template::render(
        "register",
        context! { active_page: "register", step: "invitation_code", form: context! {} },
    )
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

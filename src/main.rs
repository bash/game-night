use database::SqliteRepository;
use email::{EmailSender, EmailSenderImpl};
use rocket::error;
use rocket::fairing::{self, Fairing};
use rocket::fs::FileServer;
use rocket::{get, launch, routes, FromForm};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};

mod database;
mod email;
mod invitation;
mod register;
mod users;

#[launch]
fn rocket() -> _ {
    rocket::build()
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
        .attach(invite_admin_user())
        .attach(initialize_email_sender())
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
    Template::render("register", context! { active_page: "register" })
}

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

fn invite_admin_user() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Invite Admin User", |rocket| {
        Box::pin(async {
            let connection = GameNightDatabase::fetch(rocket)
                .expect("TODO")
                .get()
                .await
                .expect("TODO");
            invitation::invite_admin_user(&mut SqliteRepository(connection))
                .await
                .unwrap();
        })
    })
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

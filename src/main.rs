use database::SqliteRepository;
use rocket::fairing::{self, Fairing};
use rocket::fs::FileServer;
use rocket::{get, launch, routes};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};

mod database;
mod invitation;
mod users;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, invite, register])
        .mount("/", FileServer::from("public"))
        .attach(Template::fairing())
        .attach(GameNightDatabase::init())
        .attach(invite_admin_user())
}

#[get("/")]
fn index() -> Template {
    Template::render("index", context! { active_page: "home" })
}

#[get("/invite")]
fn invite() -> Template {
    Template::render("invite", context! { active_page: "invite" })
}

#[get("/register")]
fn register() -> Template {
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

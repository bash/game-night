use database::SqliteRepository;
use rocket::fairing::{self, Fairing};
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::{get, launch, post, routes, FromForm};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};

mod database;
mod invitation;
mod users;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![get_index_page, get_invite_page, get_register_page, register],
        )
        .mount("/", FileServer::from("public"))
        .attach(Template::fairing())
        .attach(GameNightDatabase::init())
        .attach(invite_admin_user())
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

#[post("/register", data = "<form>")]
fn register(form: Form<RegisterForm<'_>>) -> String {
    form.words.join(" ")
}

#[derive(FromForm)]
pub(crate) struct RegisterForm<'r> {
    words: Vec<&'r str>,
    name: Option<&'r str>,
    email_address: Option<&'r str>,
    email_verification_code: Option<&'r str>,
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

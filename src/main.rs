use rocket::fs::FileServer;
use rocket::{get, launch, routes};
use rocket_dyn_templates::{context, Template};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, invite, register])
        .mount("/", FileServer::from("public"))
        .attach(Template::fairing())
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

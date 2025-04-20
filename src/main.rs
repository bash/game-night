use anyhow::{anyhow, Context as _, Result};
use database::Repository;
use login::RocketSecretKey;
use poll::poll_finalizer;
use pruning::database_pruning;
use rand::rng;
use result::HttpResult;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::request::FromRequest;
use rocket::{catch, catchers, error, get, routes, Config, Orbit, Request, Rocket, Route};
use rocket_db_pools::{sqlx::SqlitePool, Database};
use rocket_dyn_templates::{context, Template};
use socket_activation::listener_from_env;
use template::configure_template_engines;
use template::PageBuilder;

mod response;
mod services;
mod uri;

mod auth;
mod database;
mod decorations;
mod email;
mod event;
mod fmt;
mod fs;
mod groups;
mod invitation;
mod iso_8601;
mod login;
mod play;
mod poll;
mod pruning;
mod push;
mod register;
mod result;
mod socket_activation;
#[cfg(target_os = "linux")]
mod systemd;
mod template;
mod users;

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rocket = rocket::custom(figment()?);

    #[cfg(target_os = "linux")]
    let rocket = rocket.attach(systemd::SystemdNotify);

    let rocket = rocket
        .mount("/", routes![home_page])
        .mount("/", invitation::routes())
        .mount("/", register::routes())
        .mount("/", poll::routes())
        .mount("/", play::routes())
        .mount("/", users::routes())
        .mount("/", login::routes())
        .mount("/", event::routes())
        .mount("/", push::routes())
        .register("/", login::catchers())
        .register("/", auth::catchers())
        .register("/", catchers![not_found])
        .mount("/", file_server())
        .attach(Template::try_custom(configure_template_engines))
        .attach(GameNightDatabase::init())
        .attach(email::email_sender_fairing())
        .attach(invite_admin_user())
        .attach(login::auto_login_fairing())
        .attach(poll_finalizer())
        .attach(database_pruning())
        .attach(users::LastActivity)
        .attach(push::web_push_fairing());

    if let Some(b) = listener_from_env()? {
        rocket.launch_on(b).await?;
    } else {
        rocket.launch().await?;
    }

    Ok(())
}

fn figment() -> Result<Figment> {
    let figment = Config::figment();
    let secret_keys_path: String = figment.extract_inner("secret_keys_path")?;
    let key = RocketSecretKey::read_or_generate(secret_keys_path, &mut rng()).unwrap();
    Ok(figment.merge((rocket::Config::SECRET_KEY, &key.0)))
}

#[cfg(feature = "serve-static-files")]
fn file_server() -> impl Into<Vec<Route>> {
    // The goal here is that the file server is always checked first,
    // so that Forwards from User or AuthorizedTo guards
    // are not overruled by the file server's Forward(404).
    rocket::fs::FileServer::new("public").rank(-100)
}

#[cfg(not(feature = "serve-static-files"))]
fn file_server() -> impl Into<Vec<Route>> {
    routes![]
}

#[get("/", rank = 20)]
fn home_page(page: PageBuilder<'_>) -> Template {
    page.render(
        "index",
        context! { getting_invited_uri: uri!(register::getting_invited_page())},
    )
}

#[catch(404)]
async fn not_found(request: &Request<'_>) -> HttpResult<Template> {
    let page = PageBuilder::from_request(request)
        .await
        .success_or_else(|| anyhow!("failed to create page builder"))?;
    Ok(page.render("errors/404", ()))
}

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

fn invite_admin_user() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Invite Admin User", |rocket| {
        Box::pin(async move {
            if let Err(e) = try_invite_admin_user(rocket).await {
                error!("{:?}", e);
            }
        })
    })
}

async fn try_invite_admin_user(rocket: &Rocket<Orbit>) -> Result<()> {
    use crate::services::RocketResolveExt as _;
    let mut repository: Box<dyn crate::database::Repository> = rocket.resolve().await?;
    invitation::invite_admin_user(&mut *repository)
        .await
        .context("failed to invite admin user")
}

fn default<T: Default>() -> T {
    T::default()
}

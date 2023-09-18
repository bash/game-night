use anyhow::{Context, Result};
use database::Repository;
use email::{EmailSender, EmailSenderImpl};
use keys::GameNightKeys;
use poll::poll_finalizer;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{
    async_trait, catch, catchers, error, get, launch, routes, Build, Config, Phase, Request,
    Rocket, Route,
};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Engines, Template};
use serde::Deserialize;
use std::collections::HashMap;
use template::{PageBuilder, PageType};
use users::User;

mod authentication;
mod authorization;
mod database;
mod email;
mod email_verification_code;
mod emails;
mod invitation;
mod keys;
mod login;
mod poll;
mod register;
mod template;
mod users;

#[launch]
fn rocket() -> _ {
    rocket::custom(figment())
        .mount("/", routes![get_index_page, get_play_page])
        .mount("/", invitation::routes())
        .mount("/", register::routes())
        .mount("/", poll::routes())
        .mount("/", users::routes())
        .mount("/", login::routes())
        .register("/", login::catchers())
        .register("/", authorization::catchers())
        .register("/", catchers![not_found])
        .mount("/", file_server())
        .attach(Template::custom(configure_template_engines))
        .attach(GameNightDatabase::init())
        .attach(initialize_email_sender())
        .attach(invite_admin_user())
        .attach(login::auto_login_fairing())
        .attach(poll_finalizer())
}

fn figment() -> Figment {
    let keys = GameNightKeys::read_or_generate().unwrap();
    Config::figment().merge(("secret_key", &keys.rocket_secret_key))
}

#[cfg(debug_assertions)]
fn file_server() -> impl Into<Vec<Route>> {
    rocket::fs::FileServer::from("public")
}

#[cfg(not(debug_assertions))]
fn file_server() -> impl Into<Vec<Route>> {
    routes![]
}

#[get("/")]
fn get_index_page(page: PageBuilder<'_>) -> Template {
    page.render("index", context! {})
}

#[get("/play")]
fn get_play_page(page: PageBuilder<'_>, _user: User) -> Template {
    page.type_(PageType::Play).render("play", context! {})
}

#[catch(404)]
async fn not_found(request: &Request<'_>) -> Template {
    let page = PageBuilder::from_request(request)
        .await
        .expect("Page builder guard is infallible");
    let type_ = request.uri().try_into().unwrap_or_default();
    page.type_(type_).render("errors/404", ())
}

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct UrlPrefix<'a>(pub(crate) Absolute<'a>);

#[async_trait]
impl<'r> FromRequest<'r> for UrlPrefix<'r> {
    type Error = rocket::figment::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.rocket().figment().extract_inner("url_prefix") {
            Ok(value) => Outcome::Success(UrlPrefix(value)),
            Err(e) => Outcome::Failure((Status::InternalServerError, e)),
        }
    }
}

fn invite_admin_user() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Invite Admin User", |rocket| {
        Box::pin(async {
            match try_invite_admin_user(&rocket).await {
                Ok(_) => Ok(rocket),
                Err(e) => {
                    error!("{:?}", e);
                    Err(rocket)
                }
            }
        })
    })
}

async fn try_invite_admin_user(rocket: &Rocket<Build>) -> Result<()> {
    invitation::invite_admin_user(&mut *rocket.repository().await?)
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

fn configure_template_engines(engines: &mut Engines) {
    engines.tera.register_filter("markdown", markdown_filter);
}

fn markdown_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    use pulldown_cmark::{html, Options, Parser};

    const OPTIONS: Options = Options::empty()
        .union(Options::ENABLE_TABLES)
        .union(Options::ENABLE_FOOTNOTES)
        .union(Options::ENABLE_STRIKETHROUGH);

    let input = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("This filter expects a string as input"))?;

    let parser = Parser::new_ext(input, OPTIONS);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(html_output.into())
}

#[async_trait]
trait RocketExt {
    async fn repository(&self) -> Result<Box<dyn Repository>>;
}

#[async_trait]
impl<P: Phase> RocketExt for Rocket<P> {
    async fn repository(&self) -> Result<Box<dyn Repository>> {
        let database = GameNightDatabase::fetch(self)
            .context("database not configured")?
            .get()
            .await
            .context("failed to retrieve database")?;
        Ok(database::create_repository(database))
    }
}

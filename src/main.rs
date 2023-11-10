use anyhow::{Context as _, Error, Result};
use database::Repository;
use email::{EmailSender, EmailSenderImpl};
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
use rocket_dyn_templates::{context, Template};
use serde::Deserialize;
use template::configure_template_engines;
use template::PageBuilder;

mod auth;
mod database;
mod email;
mod event;
mod invitation;
mod login;
mod play;
mod poll;
mod register;
#[cfg(target_os = "linux")]
mod systemd;
mod template;
mod users;

#[launch]
fn rocket() -> _ {
    let rocket = rocket::custom(figment());

    #[cfg(target_os = "linux")]
    let rocket = rocket.attach(systemd::SystemdNotify);

    rocket
        .mount("/", routes![get_index_page])
        .mount("/", invitation::routes())
        .mount("/", register::routes())
        .mount("/", poll::routes())
        .mount("/", play::routes())
        .mount("/", users::routes())
        .mount("/", login::routes())
        .register("/", login::catchers())
        .register("/", auth::catchers())
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
    let keys = login::GameNightKeys::read_or_generate().unwrap();
    Config::figment().merge(("secret_key", &keys.rocket_secret_key))
}

#[cfg(debug_assertions)]
fn file_server() -> impl Into<Vec<Route>> {
    // The goal here is that the file server is alwaays checked first,
    // so that Forwards from User or AuthorizedTo guards
    // are not overruled by the file server's Forward(404).
    rocket::fs::FileServer::from("public").rank(-100)
}

#[cfg(not(debug_assertions))]
fn file_server() -> impl Into<Vec<Route>> {
    routes![]
}

#[get("/", rank = 20)]
fn get_index_page(page: PageBuilder<'_>) -> Template {
    page.render("index", context! {})
}

#[catch(404)]
async fn not_found(request: &Request<'_>) -> Template {
    let page = PageBuilder::from_request(request)
        .await
        .expect("Page builder guard is infallible");
    page.render("errors/404", ())
}

#[derive(Debug, Database)]
#[database("sqlite")]
pub(crate) struct GameNightDatabase(SqlitePool);

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct UrlPrefix<'a>(pub(crate) Absolute<'a>);

impl<'a> UrlPrefix<'a> {
    fn to_static(&self) -> UrlPrefix<'static> {
        UrlPrefix(Absolute::parse_owned(self.0.to_string()).unwrap())
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for UrlPrefix<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.rocket().url_prefix() {
            Ok(value) => Outcome::Success(value),
            Err(e) => Outcome::Error((Status::InternalServerError, e)),
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

#[async_trait]
trait RocketExt {
    async fn repository(&self) -> Result<Box<dyn Repository>>;

    fn url_prefix(&self) -> Result<UrlPrefix<'_>>;

    fn email_sender(&self) -> Result<Box<dyn EmailSender>>;
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

    fn url_prefix(&self) -> Result<UrlPrefix<'_>> {
        self.figment()
            .extract_inner("url_prefix")
            .map(UrlPrefix)
            .map_err(Into::into)
    }

    fn email_sender(&self) -> Result<Box<dyn EmailSender>> {
        self.state::<Box<dyn EmailSender>>()
            .context("email sender not configured")
            .map(Clone::clone)
    }
}

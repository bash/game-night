use anyhow::{Context as _, Result};
use database::Repository;
use email::{EmailSender, EmailSenderImpl};
use event::{EventEmailSender, EventLifecycle};
use login::RocketSecretKey;
use poll::poll_finalizer;
use pruning::database_pruning;
use rand::thread_rng;
use rocket::fairing::{self, Fairing};
use rocket::figment::Figment;
use rocket::request::FromRequest;
use rocket::{catch, catchers, error, get, routes, Build, Config, Phase, Request, Rocket, Route};
use rocket_db_pools::{sqlx::SqlitePool, Database, Pool};
use rocket_dyn_templates::{context, Template};
use socket_activation::listener_from_env;
use template::configure_template_engines;
use template::PageBuilder;

mod uri;

mod auth;
mod database;
mod decorations;
mod email;
mod event;
mod fmt;
mod fs;
mod invitation;
mod iso_8601;
mod login;
mod play;
mod poll;
mod pruning;
mod register;
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
        .register("/", login::catchers())
        .register("/", auth::catchers())
        .register("/", catchers![not_found])
        .mount("/", file_server())
        .attach(Template::try_custom(configure_template_engines))
        .attach(GameNightDatabase::init())
        .attach(initialize_email_sender())
        .attach(invite_admin_user())
        .attach(login::auto_login_fairing())
        .attach(poll_finalizer())
        .attach(database_pruning())
        .attach(users::LastActivity);

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
    let key = RocketSecretKey::read_or_generate(secret_keys_path, &mut thread_rng()).unwrap();
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
async fn not_found(request: &Request<'_>) -> Template {
    let page = PageBuilder::from_request(request)
        .await
        .expect("Page builder guard is infallible");
    page.render("errors/404", ())
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
                    error!("failed to initialize email sender:\n{:?}", error);
                    Err(rocket)
                }
            }
        })
    })
}

pub(crate) trait RocketExt {
    async fn repository(&self) -> Result<Box<dyn Repository>>;

    fn email_sender(&self) -> Result<Box<dyn EmailSender>>;

    async fn event_email_sender<L: EventLifecycle>(&self) -> Result<Box<dyn EventEmailSender<L>>>;
}

impl<P: Phase> RocketExt for Rocket<P> {
    async fn repository(&self) -> Result<Box<dyn Repository>> {
        let database = GameNightDatabase::fetch(self)
            .context("database not configured")?
            .get()
            .await
            .context("failed to retrieve database")?;
        Ok(database::create_repository(database))
    }

    fn email_sender(&self) -> Result<Box<dyn EmailSender>> {
        self.state::<Box<dyn EmailSender>>()
            .context("email sender not configured")
            .cloned()
    }

    async fn event_email_sender<L: EventLifecycle>(&self) -> Result<Box<dyn EventEmailSender<L>>> {
        let repository = self.repository().await?;
        let email_sender = self.email_sender()?;
        Ok(event::create_event_email_sender(
            repository.into_event_emails_repository(),
            email_sender,
        ))
    }
}

fn default<T: Default>() -> T {
    T::default()
}

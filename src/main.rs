mod email_template;
mod response;
mod services;
mod uri;

mod auth;
mod database;
mod decorations;
mod email;
mod error_pages;
mod event;
mod fmt;
mod fs;
mod groups;
mod home;
mod infra;
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
    use rocket_db_pools::Database as _;

    let rocket = rocket::custom(crate::infra::figment()?);

    #[cfg(target_os = "linux")]
    let rocket = rocket.attach(systemd::SystemdNotify);

    let rocket = rocket
        .mount("/", rocket::routes![home::home_page])
        .mount("/", invitation::routes())
        .mount("/", register::routes())
        .mount("/", poll::routes())
        .mount("/", play::routes())
        .mount("/", users::routes())
        .mount("/", login::routes())
        .mount("/", event::routes())
        .mount("/", push::routes())
        .register("/", login::catchers())
        .register("/", error_pages::catchers())
        .attach(database::GameNightDatabase::init())
        .attach(email::email_sender_fairing())
        .attach(users::invite_admin_user_fairing())
        .attach(login::auto_login_fairing())
        .attach(poll::poll_finalizer())
        .attach(pruning::database_pruning())
        .attach(users::LastActivity)
        .attach(push::web_push_fairing())
        .attach(template::template_fairing())
        .manage(infra::HttpClient::new());

    if let Some(b) = socket_activation::listener_from_env()? {
        rocket.launch_on(b).await?;
    } else {
        rocket.launch().await?;
    }

    Ok(())
}

fn default<T: Default>() -> T {
    T::default()
}

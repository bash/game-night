use anyhow::Result;
use rocket::fairing::{self, Fairing};
use rocket::{error, routes, uri, Build, Rocket, Route};

mod key;
pub(crate) use key::*;
mod subscription;
use rocket::http::uri::Origin;
pub(crate) use subscription::*;
mod manage;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        manage::get_public_key,
        manage::subscribe,
        manage::unsubscribe
    ]
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct PushEndpoints {
    get_public_key: Origin<'static>,
    subscribe: Origin<'static>,
    unsubscribe: Origin<'static>,
}

impl Default for PushEndpoints {
    fn default() -> Self {
        Self {
            get_public_key: uri!(manage::get_public_key),
            subscribe: uri!(manage::subscribe),
            unsubscribe: uri!(manage::unsubscribe),
        }
    }
}

pub(crate) fn web_push_fairing() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Web Push", |rocket| {
        Box::pin(async {
            let rocket = rocket.manage(PushEndpoints::default());
            match read_or_generate_key(&rocket) {
                Ok(key) => Ok(rocket.manage(key)),
                Err(error) => {
                    error!("failed to initialize web push:\n{:?}", error);
                    Err(rocket)
                }
            }
        })
    })
}

fn read_or_generate_key(rocket: &Rocket<Build>) -> Result<WebPushKey> {
    let secret_keys_path: String = rocket.figment().extract_inner("secret_keys_path")?;
    WebPushKey::read_or_generate(secret_keys_path)
}

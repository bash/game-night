use anyhow::Result;
use rocket::fairing::{self, Fairing};
use rocket::{error, Build, Rocket};

mod key;
pub(crate) use key::*;

pub(crate) fn web_push_fairing() -> impl Fairing {
    fairing::AdHoc::try_on_ignite("Web Push", |rocket| {
        Box::pin(async {
            match read_or_generate_key(&rocket) {
                Ok(key) => Ok(rocket.manage(Box::new(key))),
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

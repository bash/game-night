use crate::users::User;
use anyhow::Result;
use contact::VapidContact;
use rocket::fairing::{self, Fairing};
use rocket::http::uri::Origin;
use rocket::response::Redirect;
use rocket::{error, post, routes, uri, Build, Rocket, Route};

mod key;
pub(crate) use key::*;
mod subscription;
pub(crate) use subscription::*;
mod manage;
mod sender;
pub(crate) use sender::*;
mod contact;
mod notification;
pub(crate) use notification::*;
mod notifications;
mod testbed;
use crate::decorations::Random;
use crate::result::HttpResult;
pub(crate) use notifications::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        manage::get_public_key,
        manage::subscribe,
        manage::unsubscribe,
        testbed::testbed,
        testbed::send_push_notification,
        self_test,
    ]
}

#[post("/users/push/self-test")]
pub(crate) async fn self_test(user: User, mut push_sender: PushSender) -> HttpResult<Redirect> {
    let notification = SelfTestNotification {
        user: &user,
        random: Random::default(),
    };
    push_sender.send_templated(&notification, user.id).await?;
    Ok(Redirect::to(uri!(crate::register::profile())))
}

#[derive(Debug, Clone, serde::Serialize)]
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
            let rocket = match VapidContact::from_figment(rocket.figment()) {
                Ok(key) => rocket.manage(key),
                Err(error) => {
                    error!("failed to initialize web push:\n{:?}", error);
                    return Err(rocket);
                }
            };
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

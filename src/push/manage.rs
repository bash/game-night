use super::{PushSubscription, PushSubscriptionKeys, WebPushKey};
use crate::database::New;
use crate::users::{User, UserId};
use crate::{HttpResult, Repository};
use base64::prelude::{Engine as _, BASE64_URL_SAFE_NO_PAD};
use rocket::serde::json::Json;
use rocket::{get, post};

#[get("/_api/push/public-key")]
pub(crate) fn get_public_key(push_key: WebPushKey) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(push_key.public_key())
}

#[post("/_api/push/subscribe", format = "json", data = "<form>")]
pub(crate) async fn subscribe(
    user: User,
    form: Json<SubscribeData>,
    mut repository: Box<dyn Repository>,
) -> HttpResult<()> {
    repository
        .add_push_subscription(form.into_inner().into_subscription(user.id))
        .await?;
    Ok(())
}

#[post("/_api/push/unsubscribe", format = "json", data = "<form>")]
pub(crate) async fn unsubscribe(
    user: User,
    form: Json<UnsubscribeData>,
    mut repository: Box<dyn Repository>,
) -> HttpResult<()> {
    repository
        .remove_push_subscription(user.id, &form.endpoint)
        .await?;
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SubscribeData {
    endpoint: String,
    keys: PushSubscriptionKeys,
}

impl SubscribeData {
    fn into_subscription(self, user_id: UserId) -> PushSubscription<New> {
        PushSubscription {
            id: (),
            endpoint: self.endpoint,
            keys: self.keys.into(),
            expiration_time: None,
            user_id,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct UnsubscribeData {
    endpoint: String,
}

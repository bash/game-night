use crate::database::Materialized;
use crate::entity_state;
use crate::users::UserId;
use anyhow::Result;
use base64::prelude::{Engine as _, BASE64_URL_SAFE_NO_PAD};
use sqlx::types::Json;
use time::OffsetDateTime;
use web_push::p256::PublicKey;
use web_push::Auth;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct PushSubscription<S: PushSubscriptionState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) user_id: UserId,
    /// Note: the endpoint is a unique identifier for the push subscription.
    /// From the [Web Push Spec](https://w3c.github.io/push-api/#push-subscription):
    /// > A push endpoint MUST uniquely identify the push subscription.
    pub(crate) endpoint: String,
    pub(crate) keys: Json<PushSubscriptionKeys>,
    pub(crate) expiration_time: Option<OffsetDateTime>,
}

entity_state! {
    pub(crate) trait PushSubscriptionState {
        type Id = () => i64 => i64;
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub(crate) struct PushSubscriptionKeys {
    pub(crate) auth: String,
    pub(crate) p256dh: String,
}

impl PushSubscriptionKeys {
    pub(crate) fn public_key(&self) -> Result<PublicKey> {
        Ok(PublicKey::from_sec1_bytes(
            &BASE64_URL_SAFE_NO_PAD.decode(&self.p256dh)?,
        )?)
    }

    pub(crate) fn auth(&self) -> Result<Auth> {
        Ok(Auth::clone_from_slice(
            &BASE64_URL_SAFE_NO_PAD.decode(&self.auth)?,
        ))
    }
}

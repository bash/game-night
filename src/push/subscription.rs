use crate::database::Materialized;
use crate::entity_state;
use crate::users::UserId;
use sqlx::types::Json;
use time::OffsetDateTime;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
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

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub(crate) struct PushSubscriptionKeys {
    pub(crate) auth: String,
    pub(crate) p256dh: String,
}

entity_state! {
    pub(crate) trait PushSubscriptionState {
        type Id = () => i64 => i64;
    }
}

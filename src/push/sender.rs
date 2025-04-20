use super::{PushMessage, PushSubscription, VapidContact, WebPushKey};
use crate::auto_resolve;
use crate::database::Repository;
use crate::infra::HttpClient;
use crate::users::UserId;
use anyhow::Result;
use rocket::tokio::sync::Mutex as TokioMutex;
use std::sync::Arc;
use web_push::WebPushBuilder;

auto_resolve! {
    pub(crate) struct PushSender {
        repository: Arc<TokioMutex<Box<dyn Repository>>>,
        http_client: HttpClient,
        contact: VapidContact,
        key: WebPushKey,
    }
}

impl PushSender {
    pub(crate) async fn send(&mut self, message: &PushMessage, user_id: UserId) -> Result<()> {
        let mut repository = self.repository.lock().await;
        let subscriptions = repository.get_push_subscriptions(user_id).await?;
        for subscription in subscriptions {
            self.send_to_subscription(message, &subscription).await?;
        }
        Ok(())
    }

    async fn send_to_subscription(
        &self,
        message: &PushMessage,
        subscription: &PushSubscription,
    ) -> Result<()> {
        let content = serde_json::to_vec(message)?;
        let request = self.request_for(&content, subscription)?;
        let request = reqwest::Request::try_from(request)?;
        let _response = self
            .http_client
            .execute(request)
            .await?
            .error_for_status()?;
        // TODO: deal with users who have unsubscribed
        Ok(())
    }

    fn request_for(
        &self,
        content: &[u8],
        subscription: &PushSubscription,
    ) -> Result<http::Request<Vec<u8>>> {
        Ok(WebPushBuilder::new(
            subscription.endpoint.parse()?,
            subscription.keys.public_key()?,
            subscription.keys.auth()?,
        )
        .with_vapid(self.key.as_key_pair(), &self.contact.0)
        .build(content)?)
    }
}

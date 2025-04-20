use super::{NotificationRenderer, PushMessage, PushSubscription, VapidContact, WebPushKey};
use crate::auto_resolve;
use crate::database::Repository;
use crate::infra::HttpClient;
use crate::users::UserId;
use anyhow::Result;
use http::StatusCode;
use rocket::tokio::sync::Mutex as TokioMutex;
use serde::Serialize;
use std::sync::Arc;
use web_push::WebPushBuilder;

auto_resolve! {
    pub(crate) struct PushSender {
        repository: Arc<TokioMutex<Box<dyn Repository>>>,
        http_client: HttpClient,
        contact: VapidContact,
        key: WebPushKey,
        renderer: NotificationRenderer,
    }
}

impl PushSender {
    pub(crate) async fn send(&mut self, message: &PushMessage, user_id: UserId) -> Result<()> {
        for subscription in self.get_subscriptions(user_id).await? {
            self.send_to_subscription(message, &subscription).await?;
        }
        Ok(())
    }

    pub(crate) async fn send_templated(
        &mut self,
        template_name: &str,
        context: impl Serialize,
        user_id: UserId,
    ) -> Result<()> {
        let notification = self.renderer.render(template_name, context)?;
        self.send(&PushMessage::from(notification), user_id).await
    }

    async fn get_subscriptions(&mut self, user_id: UserId) -> Result<Vec<PushSubscription>> {
        let mut repository = self.repository.lock().await;
        repository.get_push_subscriptions(user_id).await
    }

    async fn remove_subscription(&mut self, user_id: UserId, endpoint: &str) -> Result<()> {
        let mut repository = self.repository.lock().await;
        repository.remove_push_subscription(user_id, endpoint).await
    }

    async fn send_to_subscription(
        &mut self,
        message: &PushMessage,
        subscription: &PushSubscription,
    ) -> Result<()> {
        let content = serde_json::to_vec(message)?;
        let request = self.request_for(&content, subscription)?;
        let request = reqwest::Request::try_from(request)?;
        let response = self.http_client.execute(request).await?;

        let status = response.status();
        if status == StatusCode::GONE || status == StatusCode::NOT_FOUND {
            self.remove_subscription(subscription.user_id, &subscription.endpoint)
                .await?;
        } else {
            _ = response.error_for_status()?;
        }
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

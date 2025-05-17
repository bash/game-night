use super::{Notification, PushMessage, PushSender};
use crate::auth::{AuthorizedTo, ManageUsers};
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::users::{User, UserId, UserQueries};
use anyhow::Error;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::{get, post, FromForm};

const DEFAULT_NOTIFICATION: &str = r#"{
    "title": "Tau's Game Night is happening on November 25 🥳",
    "body": "You're warmly invited to the next Game Night on November 25, be sure to save the date :)",
    "icon": "https://tau.garden/favicon.svg",
    "navigate": "/play",
    "requireInteraction": true,
    "actions": [
        {
            "action": "save",
            "title": "Save to Calendar",
            "navigate": "/play/event.ics"
        }
    ]
}"#;

#[get("/users/push")]
pub(crate) async fn testbed(
    admin: AuthorizedTo<ManageUsers>,
    mut users: UserQueries,
    page: PageContextBuilder<'_>,
) -> HttpResult<Templated<TestbedPage>> {
    let users = users.active().await?;
    Ok(Templated(TestbedPage {
        notification: DEFAULT_NOTIFICATION.to_string(),
        users,
        recipient_id: admin.id,
        ctx: page.build(),
    }))
}

#[post("/users/push", data = "<form>")]
pub(crate) async fn send_push_notification(
    _admin: AuthorizedTo<ManageUsers>,
    form: Form<SendPushNotificationData>,
    mut users: UserQueries,
    mut sender: PushSender,
    page: PageContextBuilder<'_>,
) -> HttpResult<Templated<TestbedPage>> {
    let form = form.into_inner();
    let message = PushMessage::from(form.notification.into_inner());
    sender.send(&message, form.recipient).await?;

    let users = users.active().await?;
    let notification = serde_json::to_string_pretty(&message.notification).map_err(Error::from)?;
    Ok(Templated(TestbedPage {
        notification,
        users,
        recipient_id: form.recipient,
        ctx: page.build(),
    }))
}

#[derive(Debug, FromForm)]
pub(crate) struct SendPushNotificationData {
    recipient: UserId,
    notification: Json<Notification>,
}

#[derive(Debug, Template)]
#[template(path = "web-push/testbed.html")]
pub(crate) struct TestbedPage {
    users: Vec<User>,
    recipient_id: UserId,
    notification: String,
    ctx: PageContext,
}

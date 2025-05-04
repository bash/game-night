use super::{Notification, PushMessage, PushSender};
use crate::auth::{AuthorizedTo, ManageUsers};
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::users::{UserId, UsersQuery};
use anyhow::Error;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::{get, post, FromForm};
use rocket_dyn_templates::{context, Template};

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
    _admin: AuthorizedTo<ManageUsers>,
    mut users: UsersQuery,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    let users = users.active().await?;
    Ok(page.render(
        "web-push/testbed",
        context! { users, notification: DEFAULT_NOTIFICATION },
    ))
}

#[post("/users/push", data = "<form>")]
pub(crate) async fn send_push_notification(
    _admin: AuthorizedTo<ManageUsers>,
    form: Form<SendPushNotificationData>,
    mut users: UsersQuery,
    mut sender: PushSender,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    let form = form.into_inner();
    let message = PushMessage::from(form.notification.into_inner());
    sender.send(&message, form.recipient).await?;
    let users = users.active().await?;
    Ok(page.render(
        "web-push/testbed",
        context! { users, notification: serde_json::to_string_pretty(&message.notification).map_err(Error::from)? },
    ))
}

#[derive(Debug, FromForm)]
pub(crate) struct SendPushNotificationData {
    recipient: UserId,
    notification: Json<Notification>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Notification {
    title: String,
    body: Option<String>,
    /// A URL to open when the user clicks the notification.
    navigate: Option<String>,
    /// A URL.
    icon: Option<String>,
    /// Indicates that a notification should remain active until the
    /// user clicks or dismisses it, rather than closing automatically.
    #[serde(rename = "requireInteraction")]
    require_interaction: Option<bool>,
    /// A boolean value specifying whether the notification should be silent,
    /// i.e., no sounds or vibrations should be issued regardless of the device settings.
    silent: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct NotificationAction {
    action: String,
    title: String,
    icon: Option<String>,
    /// A URL to open when the user clicks the notification.
    navigate: Option<String>,
}

/// See: <https://pr-preview.s3.amazonaws.com/w3c/push-api/pull/385.html#members>
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct PushMessage {
    web_push: WebPushDisambiguator,
    notification: Notification,
}

impl From<Notification> for PushMessage {
    fn from(notification: Notification) -> Self {
        Self {
            web_push: WebPushDisambiguator,
            notification,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WebPushDisambiguator;

impl serde::Serialize for WebPushDisambiguator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(8030)
    }
}

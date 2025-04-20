#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Notification {
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<String>,
    /// A URL to open when the user clicks the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) navigate: Option<String>,
    /// A URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<String>,
    /// Indicates that a notification should remain active until the
    /// user clicks or dismisses it, rather than closing automatically.
    #[serde(rename = "requireInteraction", skip_serializing_if = "Option::is_none")]
    pub(crate) require_interaction: Option<bool>,
    /// A boolean value specifying whether the notification should be silent,
    /// i.e., no sounds or vibrations should be issued regardless of the device settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) silent: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct NotificationAction {
    pub(crate) action: String,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<String>,
    /// A URL to open when the user clicks the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) navigate: Option<String>,
}

/// See: <https://pr-preview.s3.amazonaws.com/w3c/push-api/pull/385.html#members>
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct PushMessage {
    pub(crate) web_push: WebPushDisambiguator,
    pub(crate) notification: Notification,
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

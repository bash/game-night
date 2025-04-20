/// Corresponds with the [`Notification` options][opts]
/// and some extra fields from the [declarative push spec][decl].
/// Documentation copied from MDN.
///
/// [opts]: https://developer.mozilla.org/en-US/docs/Web/API/Notification/Notification#options
/// [decl]: https://pr-preview.s3.amazonaws.com/w3c/push-api/pull/385.html#members
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
    /// A URL of the image used to represent the notification when
    /// there isn't enough space to display the notification itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) badge: Option<String>,
    /// A URL of the image to be displayed in the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) image: Option<String>,
    /// Indicates that a notification should remain active until the
    /// user clicks or dismisses it, rather than closing automatically.
    #[serde(rename = "requireInteraction", skip_serializing_if = "Option::is_none")]
    pub(crate) require_interaction: Option<bool>,
    /// A boolean value specifying whether the user should be notified
    /// after a new notification replaces an old one.
    /// If `true`, then tag also must be set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) renotify: Option<bool>,
    /// A string representing an identifying tag for the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tag: Option<String>,
    /// A boolean value specifying whether the notification should be silent,
    /// i.e., no sounds or vibrations should be issued regardless of the device settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) silent: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) actions: Vec<NotificationAction>,
    /// Arbitrary data that you want associated with the notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<serde_json::Value>,
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

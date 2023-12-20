use serde::{Deserialize, Serialize};
use zbus::dbus_proxy;
use zbus::zvariant::Type;

#[derive(Debug, Deserialize, Serialize, Type)]
pub struct TextMessage {
    pub from: String,
    pub to: String,
    pub body: String,
}

#[dbus_proxy(
    interface = "garden.tau.game_night.TextMessageOutbox",
    default_service = "garden.tau.game_night.TextMessageOutbox",
    default_path = "/garden/tau/game_night/TextMessageOutbox",
    gen_blocking = false
)]
pub trait TextMessageOutbox {
    fn queue_message(&self, message: TextMessage) -> zbus::Result<()>;
}

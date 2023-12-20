use anyhow::Result;
use log::error;
use sms_outbox::TextMessage;
use std::future::pending;
use twilio::TwilioClient;
use zbus::{dbus_interface, ConnectionBuilder};

mod twilio;

struct TextMessageOutbox(TwilioClient);

#[dbus_interface(name = "garden.tau.game_night.TextMessageOutbox")]
impl TextMessageOutbox {
    async fn queue_message(&self, message: TextMessage) {
        if let Err(e) = self.0.send_message(&message).await {
            error!("failed to send message {message:?}: {e}")
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let _connection = ConnectionBuilder::session()?
        .name("garden.tau.game_night.TextMessageOutbox")?
        .serve_at(
            "/garden/tau/game_night/TextMessageOutbox",
            TextMessageOutbox(TwilioClient::from_env()?),
        )?
        .build()
        .await?;

    // handling D-Bus messages is done in the background
    pending::<()>().await;

    Ok(())
}

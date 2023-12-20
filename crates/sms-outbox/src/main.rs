use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use std::future::pending;
use twilio::TwilioClient;
use zbus::zvariant::Type;
use zbus::{dbus_interface, ConnectionBuilder};

mod twilio;

struct SmsOutbox(TwilioClient);

#[dbus_interface(name = "garden.tau.game_night.SmsOutbox1")]
impl SmsOutbox {
    async fn queue_message(&self, message: Message) {
        if let Err(e) = self.0.send_message(&message).await {
            error!("failed to send message {message:?}: {e}")
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Type)]
struct Message {
    from: String,
    to: String,
    body: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let _connection = ConnectionBuilder::session()?
        .name("garden.tau.game_night.SmsOutbox")?
        .serve_at(
            "/garden/tau/game_night/SmsOutbox",
            SmsOutbox(TwilioClient::from_env()?),
        )?
        .build()
        .await?;

    // handling D-Bus messages is done in the background
    pending::<()>().await;

    Ok(())
}

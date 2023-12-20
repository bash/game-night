use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sms_outbox::TextMessage;
use std::env;

struct TwilioCredentials {
    account_sid: String,
    auth_token: String,
}

pub(crate) struct TwilioClient {
    client: reqwest::Client,
    credentials: TwilioCredentials,
}

impl TwilioClient {
    pub(crate) fn from_env() -> Result<Self> {
        let client = reqwest::Client::new();
        let credentials = TwilioCredentials::from_env()?;
        Ok(TwilioClient {
            client,
            credentials,
        })
    }

    pub(crate) async fn send_message(&self, message: &TextMessage) -> Result<()> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.credentials.account_sid
        );
        let response = self
            .client
            .post(url)
            .basic_auth(
                &self.credentials.account_sid,
                Some(&self.credentials.auth_token),
            )
            .form(&TwilioMessage::from(message))
            .send()
            .await?;
        ensure_success(response).await
    }
}

async fn ensure_success(response: reqwest::Response) -> Result<()> {
    if !response.status().is_success() {
        let error: TwilioError = response.json().await?;
        Err(anyhow!("{} (error {})", error.message, error.code))
    } else {
        Ok(())
    }
}

impl TwilioCredentials {
    fn from_env() -> Result<Self> {
        let account_sid = env::var("TWILIO_ACCOUNT_SID")
            .context("failed to read environment variable `TWILIO_ACCOUNT_SID`")?;
        let auth_token = env::var("TWILIO_AUTH_TOKEN")
            .context("failed to read environment variable `TWILIO_AUTH_TOKEN`")?;
        Ok(Self {
            account_sid,
            auth_token,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct TwilioMessage<'a> {
    from: &'a str,
    to: &'a str,
    body: &'a str,
}

impl<'a> From<&'a TextMessage> for TwilioMessage<'a> {
    fn from(TextMessage { from, to, body }: &'a TextMessage) -> Self {
        Self { from, to, body }
    }
}

#[derive(Debug, Deserialize)]
struct TwilioError {
    code: i64,
    message: String,
}

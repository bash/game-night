use crate::Message;
use anyhow::{anyhow, bail, Context, Result};
use http_auth_basic::Credentials;
use serde::{Deserialize, Serialize};
use std::env;
use surf::Body;

struct TwilioCredentials {
    account_sid: String,
    auth_token: String,
}

pub(crate) struct TwilioClient {
    client: surf::Client,
    credentials: TwilioCredentials,
}

impl TwilioClient {
    pub(crate) fn from_env() -> Result<Self> {
        let client = surf::Client::new();
        let credentials = TwilioCredentials::from_env()?;
        Ok(TwilioClient {
            client,
            credentials,
        })
    }

    pub(crate) async fn send_message(&self, message: &Message) -> Result<()> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.credentials.account_sid
        );
        let mut res = self
            .client
            .post(url)
            .header(
                "Authorization",
                Credentials::from(&self.credentials).as_http_header(),
            )
            .body(Body::from_form(&TwilioMessage::from(message)).map_err(to_anyhow)?)
            .send()
            .await
            .map_err(to_anyhow)?;
        if !res.status().is_success() {
            let error: TwilioError = res.body_json().await.map_err(to_anyhow)?;
            bail!("{} (error {})", error.message, error.code)
        }
        Ok(())
    }
}

fn to_anyhow(error: surf::Error) -> anyhow::Error {
    anyhow!(error)
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

impl<'a> From<&'a Message> for TwilioMessage<'a> {
    fn from(Message { from, to, body }: &'a Message) -> Self {
        Self { from, to, body }
    }
}

impl<'a> From<&'a TwilioCredentials> for Credentials {
    fn from(value: &'a TwilioCredentials) -> Self {
        Credentials::new(&value.account_sid, &value.auth_token)
    }
}

#[derive(Debug, Deserialize)]
struct TwilioError {
    code: i64,
    message: String,
}

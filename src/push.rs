use anyhow::Result;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request, State};
use std::io;
use web_push::jwt_simple::algorithms::{ECDSAP256KeyPairLike as _, ES256KeyPair};

pub(crate) fn read_or_generate_web_push_key_pair() -> Result<WebPushKey> {
    crate::fs::read_or_generate(&WebPushKeyFile).map(WebPushKey)
}

pub(crate) struct WebPushKey(ES256KeyPair);

impl WebPushKey {
    pub(crate) fn public_key(&self) -> Vec<u8> {
        self.0.key_pair().public_key().to_bytes_uncompressed()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for &'r WebPushKey {
    type Error = <&'r State<WebPushKey> as FromRequest<'r>>::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .guard::<&'r State<WebPushKey>>()
            .await
            .map(|s| s.inner())
    }
}

struct WebPushKeyFile;

impl crate::fs::GeneratedFile for WebPushKeyFile {
    type Value = ES256KeyPair;

    fn generate(&self) -> Self::Value {
        ES256KeyPair::generate()
    }

    fn file_name(&self) -> &'static str {
        "web-push-key.pem"
    }

    fn write(&self, value: &Self::Value, write: &mut dyn io::Write) -> Result<()> {
        Ok(write!(write, "{}", value.to_pem()?)?)
    }

    fn read(&self, read: &mut dyn io::Read) -> Result<Self::Value> {
        let pem = io::read_to_string(read)?;
        Ok(ES256KeyPair::from_pem(&pem)?)
    }
}

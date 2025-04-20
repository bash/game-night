use anyhow::Result;
use std::io;
use std::path::Path;
use web_push::jwt_simple::algorithms::ES256KeyPair;

pub(crate) struct WebPushKey(ES256KeyPair);

impl WebPushKey {
    pub(crate) fn read_or_generate(path: impl AsRef<Path>) -> Result<Self> {
        crate::fs::read_or_generate(path.as_ref(), WebPushKeyFile)
    }
}

struct WebPushKeyFile;

impl crate::fs::GeneratedFile for WebPushKeyFile {
    type Value = WebPushKey;

    fn generate(&mut self) -> Self::Value {
        WebPushKey(ES256KeyPair::generate())
    }

    fn file_name(&self) -> &'static str {
        "web-push-key.pem"
    }

    fn write(&self, value: &Self::Value, write: &mut dyn io::Write) -> Result<()> {
        Ok(write!(write, "{}", value.0.to_pem()?)?)
    }

    fn read(&self, read: &mut dyn io::Read) -> Result<Self::Value> {
        let pem = io::read_to_string(read)?;
        Ok(WebPushKey(ES256KeyPair::from_pem(&pem)?))
    }
}

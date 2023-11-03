use anyhow::Result;
use std::io;
use web_push::jwt_simple::algorithms::ES256KeyPair;

pub(crate) fn read_or_generate_web_push_key_pair() -> Result<ES256KeyPair> {
    crate::fs::read_or_generate(&WebPushKeyFile)
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

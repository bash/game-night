use crate::impl_resolve_for_state;
use anyhow::Result;
use std::io;
use std::path::Path;
use std::sync::Arc;
use web_push::jwt_simple::prelude::{ECDSAP256KeyPairLike as _, ES256KeyPair};

#[derive(Clone)]
pub(crate) struct WebPushKey(Arc<ES256KeyPair>);

impl WebPushKey {
    pub(crate) fn read_or_generate(path: impl AsRef<Path>) -> Result<Self> {
        crate::fs::read_or_generate(path.as_ref(), WebPushKeyFile)
    }

    pub(crate) fn public_key(&self) -> Vec<u8> {
        self.0.key_pair().public_key().to_bytes_uncompressed()
    }

    pub(crate) fn as_key_pair(&self) -> &ES256KeyPair {
        &self.0
    }
}

impl_resolve_for_state!(WebPushKey: "web push key");

struct WebPushKeyFile;

impl crate::fs::GeneratedFile for WebPushKeyFile {
    type Value = WebPushKey;

    fn generate(&mut self) -> Self::Value {
        WebPushKey(Arc::new(ES256KeyPair::generate()))
    }

    fn file_name(&self) -> &'static str {
        "web-push-key.pem"
    }

    fn write(&self, value: &Self::Value, write: &mut dyn io::Write) -> Result<()> {
        Ok(write!(write, "{}", value.0.to_pem()?)?)
    }

    fn read(&self, read: &mut dyn io::Read) -> Result<Self::Value> {
        let pem = io::read_to_string(read)?;
        Ok(WebPushKey(Arc::new(ES256KeyPair::from_pem(&pem)?)))
    }
}

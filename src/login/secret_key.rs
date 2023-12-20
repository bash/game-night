use anyhow::Result;
use rand::{thread_rng, RngCore};
use std::io;

#[derive(Debug)]
pub(crate) struct RocketSecretKey(pub(crate) Vec<u8>);

impl RocketSecretKey {
    pub(crate) fn read_or_generate() -> Result<Self> {
        crate::fs::read_or_generate(&RocketSecretKeyFile)
    }

    fn generate() -> Self {
        let mut bytes = vec![0; 512];
        thread_rng().fill_bytes(&mut bytes);
        Self(bytes)
    }
}

struct RocketSecretKeyFile;

impl crate::fs::GeneratedFile for RocketSecretKeyFile {
    type Value = RocketSecretKey;

    fn generate(&self) -> Self::Value {
        RocketSecretKey::generate()
    }

    fn file_name(&self) -> &'static str {
        "rocket-secret-key.pem"
    }

    fn write(&self, value: &Self::Value, write: &mut dyn io::Write) -> Result<()> {
        let pem = pem::encode_string("ROCKET SECRET KEY", pem::LineEnding::LF, &value.0)?;
        Ok(write!(write, "{}", pem)?)
    }

    fn read(&self, read: &mut dyn io::Read) -> Result<Self::Value> {
        let mut pem = Vec::new();
        read.read_to_end(&mut pem)?;
        let (_, key) = pem::decode_vec(&pem)?;
        Ok(RocketSecretKey(key))
    }
}

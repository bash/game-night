use anyhow::Result;
use rand::distr::{Distribution, StandardUniform};
use rand::Rng;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct RocketSecretKey(pub(crate) Vec<u8>);

impl Distribution<RocketSecretKey> for StandardUniform {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> RocketSecretKey {
        let mut bytes = vec![0; 512];
        rng.fill_bytes(&mut bytes);
        RocketSecretKey(bytes)
    }
}

impl RocketSecretKey {
    pub(crate) fn read_or_generate<R: Rng>(path: impl AsRef<Path>, rng: &mut R) -> Result<Self> {
        crate::fs::read_or_generate(path.as_ref(), RocketSecretKeyFile(rng))
    }
}

struct RocketSecretKeyFile<'a, R>(&'a mut R);

impl<R: Rng> crate::fs::GeneratedFile for RocketSecretKeyFile<'_, R> {
    type Value = RocketSecretKey;

    fn generate(&mut self) -> Self::Value {
        self.0.random()
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

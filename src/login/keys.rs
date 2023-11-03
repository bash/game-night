use anyhow::Result;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GameNightKeys {
    #[serde(with = "hex")]
    pub(crate) rocket_secret_key: Vec<u8>,
}

impl GameNightKeys {
    pub(crate) fn read_or_generate() -> Result<Self> {
        crate::fs::read_or_generate(&GameNightKeysFile)
    }

    fn generate() -> Self {
        Self {
            rocket_secret_key: generate_rocket_secret_key(),
        }
    }
}

struct GameNightKeysFile;

impl crate::fs::GeneratedFile for GameNightKeysFile {
    type Value = GameNightKeys;

    fn generate(&self) -> Self::Value {
        GameNightKeys::generate()
    }

    fn file_name(&self) -> &'static str {
        "generated-keys.json"
    }

    fn write(&self, value: &Self::Value, write: &mut dyn io::Write) -> Result<()> {
        Ok(json::to_writer_pretty(write, &value)?)
    }

    fn read(&self, read: &mut dyn io::Read) -> Result<Self::Value> {
        Ok(json::from_reader(read)?)
    }
}

fn generate_rocket_secret_key() -> Vec<u8> {
    let mut bytes = vec![0; 512];
    thread_rng().fill_bytes(&mut bytes);
    bytes
}

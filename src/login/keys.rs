use anyhow::{Context as _, Result};
use dirs::data_local_dir;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GameNightKeys {
    #[serde(with = "hex")]
    pub(crate) rocket_secret_key: Vec<u8>,
}

impl GameNightKeys {
    pub(crate) fn read_or_generate() -> Result<Self> {
        let file_path = get_file_path()?;
        create_dir_all(file_path.parent().unwrap())?;
        match write(Self::generate, &file_path) {
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(read(&file_path)?),
            result => Ok(result?),
        }
    }

    fn generate() -> Self {
        Self {
            rocket_secret_key: generate_rocket_secret_key(),
        }
    }
}

fn get_file_path() -> Result<PathBuf> {
    let mut file_path = data_local_dir().context("data directory not available")?;
    file_path.extend(&["taus-game-night", "generated-keys.json"]);
    Ok(file_path)
}

fn read(file_path: &Path) -> io::Result<GameNightKeys> {
    Ok(json::from_reader(File::open(file_path)?)?)
}

fn write(generate_keys: impl Fn() -> GameNightKeys, file_path: &Path) -> io::Result<GameNightKeys> {
    let writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(file_path)?;
    let keys = generate_keys();
    json::to_writer_pretty(writer, &keys)?;
    Ok(keys)
}

fn generate_rocket_secret_key() -> Vec<u8> {
    let mut bytes = vec![0; 512];
    thread_rng().fill_bytes(&mut bytes);
    bytes
}

use crate::impl_resolve_for_state;
use anyhow::{Context as _, Result};
use serde_json as json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs, io};

#[derive(Debug, Clone)]
pub(crate) struct Assets {
    pub(super) import_map: Option<String>,
    pub(super) asset_map: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SharedAssets(pub(crate) Arc<Assets>);

impl_resolve_for_state!(SharedAssets: "assets");

impl Assets {
    pub(crate) fn load() -> Result<Self> {
        Ok(Self {
            import_map: load_import_map()?,
            asset_map: load_asset_map()?,
        })
    }
}

fn load_import_map() -> Result<Option<String>> {
    read_file_next_to_exe("import-map.json")
}

fn load_asset_map() -> Result<HashMap<String, String>> {
    let json = read_file_next_to_exe("asset-map.json")?;
    Ok(json
        .map(|json| json::from_str(&json))
        .transpose()?
        .unwrap_or_default())
}

fn read_file_next_to_exe(name: impl AsRef<Path>) -> Result<Option<String>> {
    let mut path = current_exe_dir()?;
    path.push(name);
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn current_exe_dir() -> Result<PathBuf> {
    Ok(env::current_exe()?
        .parent()
        .context("path to have a parent")?
        .to_owned())
}

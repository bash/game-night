use anyhow::{Context as _, Result};
use rocket::serde::json;
use std::collections::HashMap;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tera_macros::tera;

pub(super) fn register_asset_map_functions(tera: &mut tera::Tera) -> Result<()> {
    tera.register_function("import_map", ImportMapFn(load_import_map()?));
    tera.register_function("asset", AssetFn(load_asset_map()?));
    Ok(())
}

struct ImportMapFn(Option<String>);

impl tera::Function for ImportMapFn {
    fn call(&self, _args: &HashMap<String, json::Value>) -> tera::Result<tera::Value> {
        Ok(tera::to_value(&self.0)?)
    }
}

struct AssetFn(HashMap<String, String>);

impl tera::Function for AssetFn {
    fn call(&self, args: &HashMap<String, json::Value>) -> tera::Result<tera::Value> {
        let path: String = tera::from_value(
            args.get("path")
                .ok_or_else(|| tera::Error::msg("Missing argument path"))?
                .clone(),
        )?;
        Ok(tera::to_value(self.0.get(&path).cloned().unwrap_or(path))?)
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
    Ok(current_exe()?
        .parent()
        .context("path to have a parent")?
        .to_owned())
}

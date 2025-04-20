use super::Notification;
use crate::{impl_resolve_for_state, template};
use anyhow::{Context as _, Error, Result};
use itertools::Itertools;
use rocket::figment::Figment;
use rocket_dyn_templates::tera::Tera;
use serde::Serialize;
use serde_json as json;
use std::collections::HashMap;
use std::fs;
use tera_macros::tera;
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone)]
pub(crate) struct NotificationRenderer {
    tera: Tera,
    templates: HashMap<String, json::Value>,
}

impl NotificationRenderer {
    pub(crate) fn from_figment(figment: &Figment) -> Result<Self> {
        let templates_dir: String = figment
            .focus("web_push")
            .extract_inner("template_dir")
            .context("invalid config")?;
        Self::from_templates_dir(&templates_dir)
    }

    pub(crate) fn from_templates_dir(templates_dir: &str) -> Result<Self> {
        let mut tera = Tera::default();
        crate::template::configure_tera(&mut tera).context("configure tera")?;
        let templates = scan_templates_dir(templates_dir)?;
        dbg!(&templates);
        Ok(Self { tera, templates })
    }

    pub(crate) fn render(
        &mut self,
        template_name: &str,
        context: impl Serialize,
    ) -> Result<Notification> {
        let value = self.render_raw(template_name, context)?;
        json::from_value(value).context("result is not a valid notification object")
    }

    pub(crate) fn render_raw(
        &mut self,
        template_name: &str,
        context: impl Serialize,
    ) -> Result<json::Value> {
        let template = self
            .templates
            .get(template_name)
            .context("template not found")?
            .clone();
        let context = tera::Context::from_serialize(context)?;
        render_template(template, &context, &mut self.tera)
    }
}

impl_resolve_for_state!(NotificationRenderer: "notification renderer");

fn render_template(
    template: tera::Value,
    context: &tera::Context,
    tera: &mut Tera,
) -> Result<json::Value> {
    use json::Value::*;
    match template {
        v @ Null | v @ Bool(_) | v @ Number(_) => Ok(v),
        String(s) => Ok(String(tera.render_str(&s, context)?)),
        Array(values) => Ok(Array(
            values
                .into_iter()
                .map(|v| render_template(v, context, tera))
                .try_collect()?,
        )),
        Object(map) => Ok(Object(
            map.into_iter()
                .map(|pair| render_template_key_value_pair(pair, context, tera))
                .try_collect()?,
        )),
    }
}

fn render_template_key_value_pair(
    (key, value): (String, json::Value),
    context: &tera::Context,
    tera: &mut Tera,
) -> Result<(String, json::Value)> {
    Ok((key, render_template(value, context, tera)?))
}

fn scan_templates_dir(templates_dir: &str) -> Result<HashMap<String, json::Value>> {
    read_files_from_dir(templates_dir)?
        .into_iter()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .map(|e| load_json_file(&e).map(|json| (template_identifier(&e), json)))
        .try_collect()
}

fn template_identifier(entry: &DirEntry) -> String {
    entry
        .path()
        .file_name()
        .expect("path has no file name")
        .to_str()
        .expect("not a valid UTF-8 file path")
        .replace('\\', "/")
}

fn load_json_file(entry: &DirEntry) -> Result<json::Value> {
    let file = fs::OpenOptions::new().read(true).open(entry.path())?;
    Ok(json::from_reader(file).with_context(|| format!("parsing {}", entry.path().display()))?)
}

fn read_files_from_dir(templates_dir: &str) -> Result<Vec<DirEntry>> {
    WalkDir::new(templates_dir)
        .max_depth(1)
        .into_iter()
        .filter_map_ok(keep_file)
        .try_collect()
        .map_err(Error::from)
}

fn keep_file(entry: DirEntry) -> Option<DirEntry> {
    if entry.file_type().is_file() {
        Some(entry)
    } else {
        None
    }
}

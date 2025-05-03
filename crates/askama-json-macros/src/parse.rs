use crate::Error;
use proc_macro2::Span;
use proc_macro2_diagnostics::SpanDiagnosticExt as _;
use std::{env, fs};
use syn::{Attribute, ItemStruct, LitStr, spanned::Spanned as _};

pub(crate) fn parse_input(input: &ItemStruct) -> Result<DeriveInput, Error> {
    let mut path = None;
    for attr in &input.attrs {
        path = parse_attr(attr)?;
    }

    let (path, path_span) = path.ok_or_else(|| input.span().error("No template specified"))?;
    let path = [&env::var("CARGO_MANIFEST_DIR").unwrap(), "/", &path].concat();
    let template = read_template_from_path(&path, path_span)?;
    Ok(DeriveInput {
        template,
        path: (path, path_span),
    })
}

pub(crate) struct DeriveInput {
    pub(crate) template: json::Value,
    pub(crate) path: (String, Span),
}

fn parse_attr(attr: &Attribute) -> Result<Option<(String, Span)>, Error> {
    let mut path = None;
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("path") {
            let value = meta.value()?;
            let s: LitStr = value.parse()?;
            path = Some((s.value(), s.span()));
            Ok(())
        } else {
            Err(meta.error("unsupported attribute"))
        }
    })?;
    Ok(path)
}

fn read_template_from_path(path: &str, span: Span) -> Result<json::Value, Error> {
    let reader = fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .map_err(|err| span.error(format!("Failed to open '{path}': {err}")))?;
    let template = json::from_reader(reader)
        .map_err(|err| span.error(format!("Failed to deserialize '{path}': {err}")))?;
    Ok(template)
}

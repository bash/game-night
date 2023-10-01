use serde::Deserialize;
use std::collections::HashMap;
use std::iter;
use tera::Tera;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{format_description, OffsetDateTime};

pub(crate) fn register_custom_functions(tera: &mut Tera) {
    tera.register_filter("markdown", markdown);
    tera.register_filter("time", time_format);
    tera.register_function("accent_color", accent_color);
    tera.register_function("avatar_symbol", avatar_symbol);
    tera.register_function("ps", ps_prefix);
}

tera_function! {
    fn ps_prefix(level: usize = 0) {
        Ok(tera::Value::String(
            iter::repeat("P.")
                .take(level + 1)
                .chain(iter::once("S."))
                .collect(),
        ))
    }
}

tera_function! {
    fn accent_color(index: usize) {
        const ACCENT_COLORS: &[&str] = &[
            "var(--home-color)",
            "var(--invite-color)",
            "var(--register-color)",
            "var(--poll-color)",
            "var(--play-color)"];
        Ok(tera::Value::String(ACCENT_COLORS[index % ACCENT_COLORS.len()].to_string()))
    }
}

tera_function! {
    fn avatar_symbol(index: usize) {
        const SYMBOLS: &[&str] = &[
            "☉", "☿", "♀", "🜨", "☾", "♂", "♃", "♄", "⛢", "♆", "⯓",
            "α", "β", "γ", "δ", "ε", "ζ", "η", "θ", "ι", "κ", "λ", "μ",
            "ν", "ξ", "ο", "π", "ρ", "σ", "τ", "υ", "φ", "χ", "ψ", "ω"];
        Ok(tera::Value::String(SYMBOLS[index % SYMBOLS.len()].to_string()))
    }
}

tera_filter! {
    fn markdown(input: String) {
        use pulldown_cmark::{html, Options, Parser};

        const OPTIONS: Options = Options::empty()
            .union(Options::ENABLE_TABLES)
            .union(Options::ENABLE_FOOTNOTES)
            .union(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(&input, OPTIONS);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        Ok(html_output.into())
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct OffsetDateTimeIsoFormat(#[serde(with = "time::serde::iso8601")] OffsetDateTime);

tera_filter! {
    fn time_format(input: OffsetDateTimeIsoFormat, format: String) {
        let input = input.0;
        let format = match parse_format(&format) {
            Ok(f) => f,
            Err(e) => return Err(tera::Error::msg(format!("Invalid format description: {e}"))),
        };
        match input.format(&format) {
            Ok(f) => Ok(tera::Value::String(f)),
            Err(e) => Err(tera::Error::msg(format!("Error formatting date {input}: {e}"))),
        }
    }
}

fn parse_format(
    format: &str,
) -> Result<Vec<FormatItem<'_>>, time::error::InvalidFormatDescription> {
    const DATE_FORMAT: &[FormatItem] =
        format_description!("[day padding:none].\u{00A0}[month repr:long]");
    const TIME_FORMAT: &[FormatItem] = format_description!("[hour padding:none]:[minute]");
    match format {
        "{time}" => Ok(TIME_FORMAT.to_vec()),
        "{date}" => Ok(DATE_FORMAT.to_vec()),
        _ => format_description::parse(format),
    }
}

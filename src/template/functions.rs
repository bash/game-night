use super::AccentColor;
use crate::decorations::{Hearts, SkinToneModifiers};
use crate::iso_8601::Iso8601;
use crate::users::EmailSubscription;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng, SeedableRng};
use rocket_dyn_templates::tera::{self, Tera};
use std::iter;
use std::sync::OnceLock;
use tera_macros::{tera_filter, tera_function};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{format_description, OffsetDateTime};
use time_tz::{timezones, OffsetDateTimeExt};

pub(crate) fn register_custom_functions(tera: &mut Tera) {
    tera.register_filter("markdown", markdown);
    tera.register_filter("time", time_format);
    tera.register_function("accent_color", accent_color);
    tera.register_function("ps", ps_prefix);
    tera.register_function("random_heart", random_heart);
    tera.register_function("random_skin_tone_modifier", random_skin_tone_modifier);
    tera.register_function("is_subscribed", is_subscribed);
}

tera_function! {
    fn ps_prefix(level: usize = 0) -> String {
        iter::repeat("P.")
            .take(level + 1)
            .chain(iter::once("S."))
            .collect()
    }
}

tera_function! {
    fn accent_color(index: usize) -> String {
        static SHUFFLED_SYMBOLS: OnceLock<Vec<AccentColor>> = OnceLock::new();
        let accent_colors = SHUFFLED_SYMBOLS.get_or_init(|| {
            const SEED: u64 = 0xdeadbeef;
            let mut accent_colors = AccentColor::values().to_vec();
            accent_colors.shuffle(&mut SmallRng::seed_from_u64(SEED));
            accent_colors
        });
        accent_colors[index % accent_colors.len()].css_value().to_string()
    }
}

tera_filter! {
    fn markdown(input: String) -> String {
        use pulldown_cmark::{html, Options, Parser};

        const OPTIONS: Options = Options::empty()
            .union(Options::ENABLE_TABLES)
            .union(Options::ENABLE_FOOTNOTES)
            .union(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(&input, OPTIONS);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    }
}

tera_filter! {
    fn time_format(input: Iso8601<OffsetDateTime>, format: String) -> Result<String, tera::Error> {
        let input = input.0.to_timezone(timezones::db::CET);
        let format = parse_format(&format).map_err(|e| tera::Error::msg(format!("Invalid format description: {e}")))?;
        input.format(&format).map_err(|e| tera::Error::msg(format!("Error formatting date {input}: {e}")))
    }
}

fn parse_format(
    format: &str,
) -> Result<Vec<FormatItem<'_>>, time::error::InvalidFormatDescription> {
    const DATE_FORMAT: &[FormatItem] =
        format_description!("[day padding:none].\u{2009}[month repr:long padding:none]");
    const DATE_WITH_YEAR_FORMAT: &[FormatItem] =
        format_description!("[day padding:none].\u{2009}[month repr:long]\u{2009}[year repr:full]");
    const TIME_FORMAT: &[FormatItem] = format_description!("[hour padding:none]:[minute]");
    match format {
        "{time}" => Ok(TIME_FORMAT.to_vec()),
        "{date}" => Ok(DATE_FORMAT.to_vec()),
        "{date_with_year}" => Ok(DATE_WITH_YEAR_FORMAT.to_vec()),
        _ => format_description::parse(format),
    }
}

tera_function! {
    fn random_heart() -> &'static str {
        thread_rng().sample(Hearts)
    }
}

tera_function! {
    fn random_skin_tone_modifier() -> &'static str {
        thread_rng().sample(SkinToneModifiers)
    }
}

tera_function! {
    fn is_subscribed(sub: EmailSubscription) -> bool {
        sub.is_subscribed(OffsetDateTime::now_utc().date())
    }
}

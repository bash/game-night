use crate::decorations::{Hearts, SkinToneModifiers};
use crate::event::EventId;
use crate::iso_8601::Iso8601;
use crate::template_v2::page_context::AccentColor;
use crate::users::EmailSubscription;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{rng, Rng, SeedableRng};
use rocket::uri;
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
    tera.register_function("event_page_uri", event_page_uri);
    tera.register_function("skip_poll_uri", skip_poll_uri);
    tera.register_function("event_ics_uri", event_ics_uri);
    tera.register_function("leave_event_uri", leave_event_uri);
}

tera_function! {
    fn ps_prefix(level: usize = 0) -> String {
        std::iter::repeat_n("P.", level + 1)
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
        rng().sample(Hearts)
    }
}

tera_function! {
    fn random_skin_tone_modifier() -> &'static str {
        rng().sample(SkinToneModifiers)
    }
}

tera_function! {
    fn is_subscribed(sub: EmailSubscription) -> bool {
        sub.is_subscribed(OffsetDateTime::now_utc().date())
    }
}

tera_function! {
    fn event_page_uri(event_id: i64) -> String {
        uri!(crate::event::event_page(id = event_id)).to_string()
    }
}

tera_function! {
    fn skip_poll_uri(event_id: i64) -> String {
        uri!(crate::poll::skip_poll_page(id = event_id)).to_string()
    }
}

tera_function! {
    fn event_ics_uri(event_id: i64) -> String {
        uri!(crate::play::event_ics(id = event_id)).to_string()
    }
}

tera_function! {
    fn leave_event_uri(event_id: i64) -> String {
        uri!(crate::event::leave_page(id = event_id)).to_string()
    }
}

use std::collections::HashMap;
use std::iter;
use tera::Tera;

pub(crate) fn register_custom_functions(tera: &mut Tera) {
    tera.register_filter("markdown", markdown);
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

use super::page_context::AccentColor;
use crate::event::EventId;
use crate::uri;
use crate::users::UserId;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom as _;
use rand::SeedableRng as _;
use std::iter;
use std::sync::OnceLock;

pub(crate) fn accent_color(index: UserId) -> AccentColor {
    static SHUFFLED_SYMBOLS: OnceLock<Vec<AccentColor>> = OnceLock::new();
    let accent_colors = SHUFFLED_SYMBOLS.get_or_init(|| {
        const SEED: u64 = 0xdeadbeef;
        let mut accent_colors = AccentColor::values().to_vec();
        accent_colors.shuffle(&mut SmallRng::seed_from_u64(SEED));
        accent_colors
    });
    accent_colors[(index.0 as usize) % accent_colors.len()]
}

pub(crate) fn ps(level: usize) -> String {
    std::iter::repeat_n("P.", level + 1)
        .chain(iter::once("S."))
        .collect()
}

pub(crate) fn event_ics_uri(event_id: EventId) -> String {
    uri!(crate::play::event_ics(id = event_id)).to_string()
}

pub(crate) fn leave_event_uri(event_id: EventId) -> String {
    uri!(crate::event::leave_page(id = event_id)).to_string()
}

pub(crate) fn skip_poll_uri(event_id: EventId) -> String {
    uri!(crate::poll::skip_poll_page(id = event_id)).to_string()
}

pub(crate) fn event_page_uri(event_id: EventId) -> String {
    uri!(crate::event::event_page(id = event_id)).to_string()
}

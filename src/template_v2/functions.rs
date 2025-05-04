use crate::template::AccentColor;
use crate::users::UserId;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom as _;
use rand::SeedableRng as _;
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

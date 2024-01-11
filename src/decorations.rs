use rand::{distributions, thread_rng, Rng as _};

pub(crate) fn random_heart() -> &'static str {
    const HEARTS: &[&str] = &["❤️", "💖", "💙", "🩵", "💚", "💛", "💜", "🩷", "🧡"];
    thread_rng().sample(distributions::Slice::new(HEARTS).unwrap())
}

pub(crate) fn random_skin_tone_modifier() -> &'static str {
    const SKIN_TONE_MODIFIERS: &[&str] = &[
        "\u{1F3FB}",
        "\u{1F3FC}",
        "\u{1F3FD}",
        "\u{1F3FE}",
        "\u{1F3FF}",
        "",
    ];
    thread_rng().sample(distributions::Slice::new(SKIN_TONE_MODIFIERS).unwrap())
}

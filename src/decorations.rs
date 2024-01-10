use rand::{distributions, thread_rng, Rng as _};

pub(crate) fn random_heart() -> &'static str {
    const HEARTS: &[&str] = &["❤️", "💖", "💙", "🩵", "💚", "💛", "💜", "🩷", "🧡"];
    thread_rng().sample(distributions::Slice::new(HEARTS).unwrap())
}

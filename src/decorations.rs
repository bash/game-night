use rand::distr::{self, Distribution};
use rand::{rng, Rng as _};
use std::sync::LazyLock;

// TODO: rename
#[derive(Debug)]
pub(crate) struct Random {
    heart: LazyLock<&'static str>,
    skin_tone_modifier: LazyLock<&'static str>,
    greeting: LazyLock<&'static str>,
    closing: LazyLock<&'static str>,
}

impl Random {
    pub(crate) fn heart(&self) -> &'static str {
        &self.heart
    }

    pub(crate) fn skin_tone_modifier(&self) -> &'static str {
        &self.skin_tone_modifier
    }

    pub(crate) fn greeting(&self) -> &'static str {
        &self.greeting
    }

    pub(crate) fn closing(&self) -> &'static str {
        &self.closing
    }
}

impl Default for Random {
    fn default() -> Self {
        Self {
            heart: LazyLock::new(|| rng().sample(Hearts)),
            skin_tone_modifier: LazyLock::new(|| rng().sample(SkinToneModifiers)),
            greeting: LazyLock::new(|| rng().sample(Greetings)),
            closing: LazyLock::new(|| rng().sample(Closings)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Hearts;

impl Distribution<&'static str> for Hearts {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> &'static str {
        const HEARTS: &[&str] = &["❤️", "💖", "💙", "🩵", "💚", "💛", "💜", "🩷", "🧡"];
        rng.sample(distr::slice::Choose::new(HEARTS).unwrap())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SkinToneModifiers;

impl Distribution<&'static str> for SkinToneModifiers {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> &'static str {
        const SKIN_TONE_MODIFIERS: &[&str] = &[
            "\u{1F3FB}",
            "\u{1F3FC}",
            "\u{1F3FD}",
            "\u{1F3FE}",
            "\u{1F3FF}",
            "",
        ];
        rng.sample(distr::slice::Choose::new(SKIN_TONE_MODIFIERS).unwrap())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Greetings;

impl Distribution<&'static str> for Greetings {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> &'static str {
        const GREETINGS: &[&str] = &[
            "Hi",
            "Ciao",
            "Salü",
            "Hola",
            "Hellooo",
            "Hey there",
            "Greetings galore",
            "Aloha",
            "Howdy",
            "Hiyaa",
            "Yoohoo~",
            "Ahoy",
        ];
        rng.sample(distr::slice::Choose::new(GREETINGS).unwrap())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Closings;

impl Distribution<&'static str> for Closings {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> &'static str {
        const GREETINGS: &[&str] = &[
            "See youu~",
            "Later, Alligator",
            "In a while, Crocodile 🐊",
            "You're the best ✨",
            "XOXO",
            "Toodle-oo, Kangaroo 🦘",
            "Blow a kiss, Jellyfish 🪼",
            "Give a hug, Ladybug 🐞",
            "Goodbye, Butterfly 🦋",
            "Take care, Polar Bear 🐻‍❄️",
            "See you soon, Cute Racoon 🦝",
            "Till then, Penguin 🐧",
            "In a shake, Rattlesnake 🐍",
        ];
        rng.sample(distr::slice::Choose::new(GREETINGS).unwrap())
    }
}

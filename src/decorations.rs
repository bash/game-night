use rand::distributions::{self, Distribution};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Hearts;

impl Distribution<&'static str> for Hearts {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> &'static str {
        const HEARTS: &[&str] = &["❤️", "💖", "💙", "🩵", "💚", "💛", "💜", "🩷", "🧡"];
        rng.sample(distributions::Slice::new(HEARTS).unwrap())
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
        rng.sample(distributions::Slice::new(SKIN_TONE_MODIFIERS).unwrap())
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
        rng.sample(distributions::Slice::new(GREETINGS).unwrap())
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
        rng.sample(distributions::Slice::new(GREETINGS).unwrap())
    }
}

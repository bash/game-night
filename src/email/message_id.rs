use rand::{
    distr::{Alphanumeric, SampleString, StandardUniform},
    prelude::Distribution,
    Rng,
};
use std::fmt;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(transparent)]
pub(crate) struct MessageId(String);

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Distribution<MessageId> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MessageId {
        // https://tools.ietf.org/html/rfc5322#section-3.6.4
        MessageId(format!("<{}@{}>", generate_message_id(rng), hostname()))
    }
}

fn hostname() -> String {
    const DEFAULT_MESSAGE_ID_DOMAIN: &str = "localhost";
    hostname::get()
        .map_err(|_| ())
        .and_then(|s| s.into_string().map_err(|_| ()))
        .unwrap_or_else(|_| DEFAULT_MESSAGE_ID_DOMAIN.to_owned())
}

/// Create a random message id.
/// (Not cryptographically random)
fn generate_message_id<R: Rng + ?Sized>(rng: &mut R) -> String {
    Alphanumeric.sample_string(rng, 36)
}

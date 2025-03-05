use rand::distr::Uniform;
use rand::Rng;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct EmailVerificationCode {
    pub(crate) code: String,
    pub(crate) email_address: String,
    pub(crate) valid_until: OffsetDateTime,
}

impl EmailVerificationCode {
    pub(crate) fn generate<R: Rng>(email_address: String, rng: &mut R) -> Self {
        Self {
            code: generate_code(rng),
            email_address,
            valid_until: OffsetDateTime::now_utc() + Duration::minutes(30),
        }
    }
}

fn generate_code<R: Rng>(rng: &mut R) -> String {
    rng.sample_iter(&Uniform::try_from(1..=9).unwrap())
        .take(6)
        .map(|d| d.to_string())
        .collect()
}

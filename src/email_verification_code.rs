use rand::distributions::Uniform;
use rand::Rng;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct EmailVerificationCode {
    pub(crate) code: String,
    pub(crate) email_address: String,
    pub(crate) valid_until: OffsetDateTime,
}

impl EmailVerificationCode {
    pub(crate) fn generate(email_address: String) -> Self {
        Self {
            code: generate_code(),
            email_address,
            valid_until: OffsetDateTime::now_utc() + Duration::minutes(30),
        }
    }
}

fn generate_code() -> String {
    rand::thread_rng()
        .sample_iter(&Uniform::from(1..=9))
        .take(6)
        .map(|d| d.to_string())
        .collect()
}

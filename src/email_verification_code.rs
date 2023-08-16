use chrono::{DateTime, Duration, Local};
use rand::distributions::Alphanumeric;
use rand::Rng;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct EmailVerificationCode {
    pub(crate) code: String,
    pub(crate) email_address: String,
    pub(crate) valid_until: DateTime<Local>,
}

impl EmailVerificationCode {
    pub(crate) fn generate(email_address: String) -> Self {
        Self {
            code: generate_code(),
            email_address,
            valid_until: Local::now() + Duration::minutes(30),
        }
    }
}

fn generate_code() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect()
}

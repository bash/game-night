use crate::email::EmailMessage;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(super) struct VerificationEmail {
    pub(crate) name: String,
    pub(crate) code: String,
}

impl EmailMessage for VerificationEmail {
    fn subject(&self) -> String {
        "Let's Get You Verified".to_owned()
    }

    fn template_name(&self) -> String {
        "verification".to_owned()
    }
}

use crate::email::EmailMessage;
use serde::Serialize;
use tera::Context;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VerificationEmail {
    pub(crate) greeting: String,
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

    fn template_context(&self) -> Context {
        Context::from_serialize(self).unwrap()
    }
}

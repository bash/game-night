use crate::email::EmailMessage;
use serde::Serialize;
use tera::Context;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LoginEmail {
    pub(crate) name: String,
    pub(crate) code: String,
}

impl EmailMessage for LoginEmail {
    fn subject(&self) -> String {
        "Let's Get You Logged In".to_owned()
    }

    fn template_name(&self) -> String {
        "login".to_owned()
    }

    fn template_context(&self) -> Context {
        Context::from_serialize(self).unwrap()
    }
}

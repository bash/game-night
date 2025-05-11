use crate::decorations::Random;
use crate::email::{EmailMessage, EmailTemplateContext};
use crate::email_template;

email_template! {
    #[template(html_path = "emails/verification.html", txt_path = "emails/verification.txt")]
    #[derive(Debug)]
    pub(crate) struct VerificationEmail {
       pub(crate) code: String,
       pub(crate) name: String,
       pub(crate) random: Random,
       pub(crate) ctx: EmailTemplateContext,
    }
}

impl EmailMessage for VerificationEmail {
    fn subject(&self) -> String {
        "Let's Get You Verified".to_owned()
    }
}

use super::User;
use anyhow::Result;
use lettre::message::Mailbox;

pub(crate) trait UserMailboxExt {
    fn mailbox(&self) -> Result<Mailbox>;
}

impl UserMailboxExt for User {
    fn mailbox(&self) -> Result<Mailbox> {
        Ok(Mailbox::new(
            Some(self.name.clone()),
            self.email_address.parse()?,
        ))
    }
}

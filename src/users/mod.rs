use anyhow::Result;
use lettre::message::Mailbox;
use rocket::{routes, Route};
use time::Duration;

mod email_subscription;
pub(crate) use email_subscription::*;
mod last_activity;
pub(crate) use last_activity::*;
mod symbol;
pub(crate) use symbol::*;
mod list;
pub(crate) use list::*;
mod name;
pub(crate) use name::*;
mod admin_user;
pub(crate) use admin_user::*;
mod commands;
pub(crate) use commands::*;
mod models;
pub(crate) use models::*;
mod queries;
pub(crate) use queries::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![list::list_users]
}

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

pub(crate) const INACTIVITY_THRESHOLD: Duration = Duration::days(9 * 30);

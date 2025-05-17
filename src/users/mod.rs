use rocket::{routes, Route};

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
mod mailbox;
pub(crate) use mailbox::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![list::list_users]
}

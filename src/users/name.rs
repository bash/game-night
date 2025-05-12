use super::User;
use crate::template::prelude::*;

#[derive(Debug, Template)]
#[template(path = "users/user-name.html")]
pub(crate) struct UserNameComponent<'a> {
    user: &'a User,
    show_symbol: bool,
}

impl<'a> UserNameComponent<'a> {
    pub(crate) fn for_user(user: &'a User) -> Self {
        Self {
            user,
            show_symbol: false,
        }
    }

    pub(crate) fn with_symbol(mut self) -> Self {
        self.show_symbol = true;
        self
    }
}

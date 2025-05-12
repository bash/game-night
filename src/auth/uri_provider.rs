use crate::uri;
use crate::users::User;
use rocket::http::uri::Origin;

#[derive(Debug, Clone)]
pub(crate) struct UriProvider {
    user: User,
}

impl UriProvider {
    pub(crate) fn for_user(user: User) -> Self {
        UriProvider { user }
    }

    pub(crate) fn new_poll_page(&self) -> Option<Origin<'static>> {
        self.user
            .can_manage_poll()
            .then(|| uri!(crate::poll::new_poll_page()))
    }

    pub(crate) fn profile_page(&self) -> Origin<'static> {
        uri!(crate::register::profile())
    }

    pub(crate) fn archive_page(&self) -> Origin<'static> {
        uri!(crate::play::archive_page())
    }
}

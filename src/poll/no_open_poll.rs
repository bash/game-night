use crate::auth::UriProvider;
use crate::template_v2::prelude::*;
use crate::users::User;

#[derive(Debug, Template)]
#[template(path = "poll/no-open-poll.html")]
pub(crate) struct NoOpenPollPage {
    uri: UriProvider,
    user: User,
    ctx: PageContext,
}

impl NoOpenPollPage {
    pub(crate) fn for_user(user: User, ctx: PageContext) -> Self {
        Self {
            uri: UriProvider::for_user(user.clone()),
            user,
            ctx,
        }
    }
}

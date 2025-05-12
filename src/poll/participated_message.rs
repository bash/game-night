use super::Poll;
use crate::template::prelude::*;
use crate::users::User;

#[derive(Template, Debug)]
#[template(path = "poll/participated-message.html")]
pub(crate) struct ParticipatedMessageComponent<'a> {
    poll: &'a Poll,
    user: &'a User,
}

impl<'a> ParticipatedMessageComponent<'a> {
    pub(crate) fn for_poll(poll: &'a Poll, user: &'a User) -> Self {
        Self { poll, user }
    }
}

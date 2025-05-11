use crate::decorations::Random;
use crate::event::Event;
use crate::poll::Poll;
use crate::template::{filters, functions};
use crate::users::User;
use askama_json::JsonTemplate;

#[derive(Debug, JsonTemplate)]
#[json_template(path = "notifications/poll.json")]
pub(crate) struct PollNotification<'a> {
    pub(crate) poll: &'a Poll,
}

#[derive(Debug, JsonTemplate)]
#[json_template(path = "notifications/invited.json")]
pub(crate) struct InvitedNotification<'a> {
    pub(crate) event: &'a Event,
    pub(crate) random: Random,
}

#[derive(Debug, JsonTemplate)]
#[json_template(path = "notifications/missed.json")]
pub(crate) struct MissedNotification<'a> {
    pub(crate) event: &'a Event,
}

#[derive(Debug, JsonTemplate)]
#[json_template(path = "notifications/self-test.json")]
pub(crate) struct SelfTestNotification<'a> {
    pub(crate) user: &'a User,
    pub(crate) random: Random,
}

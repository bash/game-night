use crate::decorations::Random;
use crate::event::{Event, EventId};
use crate::poll::Poll;
use crate::template_v2::filters;
use crate::uri;
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

fn event_ics_uri(event_id: i64) -> String {
    uri!(crate::play::event_ics(id = event_id)).to_string()
}

fn leave_event_uri(event_id: i64) -> String {
    uri!(crate::event::leave_page(id = event_id)).to_string()
}

fn skip_poll_uri(event_id: i64) -> String {
    uri!(crate::poll::skip_poll_page(id = event_id)).to_string()
}

fn event_page_uri(event_id: i64) -> String {
    uri!(crate::event::event_page(id = event_id)).to_string()
}

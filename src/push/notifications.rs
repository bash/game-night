use crate::decorations::Random;
use crate::event::{Event, EventId};
use crate::poll::Poll;
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

mod filters {
    use crate::iso_8601::Iso8601;
    use askama_json::askama;
    use time::format_description::{self, BorrowedFormatItem as FormatItem};
    use time::macros::format_description;
    use time::OffsetDateTime;
    use time_tz::{timezones, OffsetDateTimeExt as _};

    pub(super) fn time(
        input: Iso8601<OffsetDateTime>,
        _: &dyn askama::Values,
        format: impl AsRef<str>,
    ) -> askama::Result<String> {
        let input = input.0.to_timezone(timezones::db::CET);
        let format = parse_format(format.as_ref())
            .map_err(|e| askama::Error::custom(format!("Invalid format description: {e}")))?;
        input
            .format(&format)
            .map_err(|e| askama::Error::custom(format!("Error formatting date {input}: {e}")))
    }

    fn parse_format(
        format: &str,
    ) -> Result<Vec<FormatItem<'_>>, time::error::InvalidFormatDescription> {
        const DATE_FORMAT: &[FormatItem] =
            format_description!("[day padding:none].\u{2009}[month repr:long padding:none]");
        const DATE_WITH_YEAR_FORMAT: &[FormatItem] = format_description!(
            "[day padding:none].\u{2009}[month repr:long]\u{2009}[year repr:full]"
        );
        const TIME_FORMAT: &[FormatItem] = format_description!("[hour padding:none]:[minute]");
        match format {
            "{time}" => Ok(TIME_FORMAT.to_vec()),
            "{date}" => Ok(DATE_FORMAT.to_vec()),
            "{date_with_year}" => Ok(DATE_WITH_YEAR_FORMAT.to_vec()),
            _ => format_description::parse(format),
        }
    }
}

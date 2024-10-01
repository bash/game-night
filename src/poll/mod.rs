use crate::auth::{AuthorizedTo, ManagePoll};
use crate::database::{Materialized, New, Repository, Unmaterialized};
use crate::entity_state;
use crate::event::{Event, EventsQuery, Polling};
use crate::iso_8601::Iso8601;
use crate::play::rocket_uri_macro_archive_page;
use crate::register::rocket_uri_macro_profile;
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::users::{User, UserId};
use anyhow::Error;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, routes, uri, FromFormField, Route, State};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Database, Decode, Encode, Sqlite, Type};
use std::fmt;
use time::OffsetDateTime;

mod finalize;
pub(crate) use finalize::*;
mod email;
use email::PollEmail;
mod new;
mod open;
pub(crate) use open::*;
mod skip;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        skip::skip_poll_page,
        skip::skip_poll,
        close_poll_page,
        close_poll,
        new::new_poll_page,
        new::new_poll,
        new::calendar,
        open::update_answers,
        email::poll_email_preview,
    ]
}

pub(crate) fn no_open_poll_page(user: User, page: PageBuilder<'_>) -> Template {
    let new_poll_uri = user.can_manage_poll().then(|| uri!(new::new_poll_page()));
    let profile_uri = uri!(profile());
    let archive_uri = uri!(archive_page());
    page.render("poll", context! { new_poll_uri, profile_uri, archive_uri })
}

#[get("/event/<id>/poll/close")]
async fn close_poll_page(
    id: i64,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::NotFound.into());
    };
    Ok(page.render(
        "poll/close",
        context! { date_selection_strategy: poll.strategy.to_string(), poll },
    ))
}

#[post("/event/<id>/poll/close")]
async fn close_poll(
    id: i64,
    user: AuthorizedTo<ManagePoll>,
    mut events: EventsQuery,
    nudge: &State<NudgeFinalizer>,
    mut repository: Box<dyn Repository>,
) -> HttpResult<Redirect> {
    let Some(poll) = events.with_id(id, &user).await?.and_then(|e| e.polling()) else {
        return Err(Status::BadRequest.into());
    };
    // open_until is inclusive, so we have to pick a date that is already in the past
    // so that we are displayed the correct page on redirect.
    // Also, we don't need subsecond precision.
    let open_until = OffsetDateTime::now_utc()
        .replace_second(0)
        .map_err(Error::from)?;
    repository
        .update_poll_open_until(poll.id, open_until)
        .await?;
    nudge.nudge();
    let event_uri = uri!(crate::event::event_page(id = poll.event.id));
    Ok(Redirect::to(event_uri))
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct Poll<S: PollState = Materialized> {
    pub(crate) id: S::Id,
    #[sqlx(try_from = "i64")]
    pub(crate) min_participants: usize,
    pub(crate) strategy: DateSelectionStrategy,
    pub(crate) open_until: Iso8601<OffsetDateTime>,
    pub(crate) stage: PollStage,
    #[sqlx(rename = "event_id")]
    pub(crate) event: S::Event,
    #[sqlx(skip)]
    pub(crate) options: S::Options,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub(crate) enum PollStage {
    Open,
    Finalizing,
    Closed,
}

entity_state! {
    pub(crate) trait PollState {
        type Id = () => i64 => i64;
        type Event = Event<Self, Polling> => i64 => Event<Self, Polling>;
        type Options: Default = Vec<PollOption<Self>> => () => Vec<PollOption<Self>>;
    }
}

impl Poll<New> {
    pub(crate) fn into_unmaterialized(self, id: i64, event_id: i64) -> Poll<Unmaterialized> {
        Poll {
            id,
            min_participants: self.min_participants,
            strategy: self.strategy,
            open_until: self.open_until,
            stage: self.stage,
            event: event_id,
            options: (),
        }
    }
}

impl Poll<Unmaterialized> {
    pub(crate) fn into_materialized(
        self,
        event: Event<Materialized, Polling>,
        options: Vec<PollOption>,
    ) -> Poll {
        Poll {
            id: self.id,
            min_participants: self.min_participants,
            strategy: self.strategy,
            open_until: self.open_until,
            stage: self.stage,
            event,
            options,
        }
    }
}

impl Poll {
    pub(crate) fn has_answer(&self, user: UserId) -> bool {
        self.options.iter().any(|o| o.has_answer(user))
    }

    pub(crate) fn has_yes_answer(&self, user: UserId) -> bool {
        self.options.iter().any(|o| o.has_yes_answer(user))
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct PollOption<S: PollOptionState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) starts_at: Iso8601<OffsetDateTime>,
    #[sqlx(skip)]
    pub(crate) answers: S::Answers,
}

entity_state! {
    pub(crate) trait PollOptionState {
        type Id = () => i64 => i64;
        type Answers: Default = Vec<Answer<Self>> => () => Vec<Answer<Self>>;
    }
}

impl<S> PollOption<S>
where
    S: AnswerState,
    S: PollOptionState<Answers = Vec<Answer<S>>>,
{
    pub(crate) fn count_yes_answers(&self) -> usize {
        self.answers.iter().filter(|a| a.value.is_yes()).count()
    }

    pub(crate) fn has_veto(&self) -> bool {
        self.answers
            .iter()
            .any(|a| matches!(a.value, AnswerValue::No { veto: true }))
    }
}

impl PollOption {
    pub(crate) fn has_answer(&self, user: UserId) -> bool {
        self.answers.iter().any(|a| a.user.id == user)
    }

    pub(crate) fn has_yes_answer(&self, user: UserId) -> bool {
        self.answers
            .iter()
            .any(|a| a.user.id == user && a.value.is_yes())
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct Answer<S: AnswerState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) value: AnswerValue,
    #[sqlx(rename = "user_id")]
    pub(crate) user: S::User,
}

entity_state! {
    pub(crate) trait AnswerState {
        type Id = () => i64 => i64;
        type User = UserId => UserId => User;
    }
}

impl Answer<Unmaterialized> {
    pub(crate) fn materialize(self, user: User) -> Answer {
        Answer {
            id: self.id,
            value: self.value,
            user,
        }
    }
}

impl<S: AnswerState> Answer<S> {
    pub(crate) fn yes(&self) -> Option<&S::User> {
        use AnswerValue::*;
        match self.value {
            No { .. } => None,
            Yes { .. } => Some(&self.user),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum AnswerValue {
    No { veto: bool },
    Yes { attendance: Attendance },
}

impl AnswerValue {
    fn no(veto: bool) -> Self {
        Self::No { veto }
    }

    fn yes(attendance: Attendance) -> Self {
        Self::Yes { attendance }
    }

    fn veto() -> Self {
        Self::no(true)
    }

    fn is_yes(self) -> bool {
        matches!(self, AnswerValue::Yes { .. })
    }

    fn to_bools(self) -> (bool, bool) {
        match self {
            AnswerValue::Yes { attendance } => (true, attendance == Attendance::Required),
            AnswerValue::No { veto } => (false, veto),
        }
    }

    fn from_bools((yes, strong): (bool, bool)) -> Self {
        match (yes, strong) {
            (true, true) => Self::yes(Attendance::Required),
            (true, false) => Self::yes(Attendance::Optional),
            (false, veto) => Self::no(veto),
        }
    }

    fn ensure_weak(self) -> Self {
        match self {
            Self::Yes { .. } => Self::yes(Attendance::Optional),
            Self::No { .. } => Self::no(false),
        }
    }
}

impl Type<Sqlite> for AnswerValue {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <&str as Type<Sqlite>>::compatible(ty)
    }
}

impl<'q, DB: Database> Encode<'q, DB> for AnswerValue
where
    &'q str: Encode<'q, DB>,
{
    fn encode_by_ref(&self, buf: &mut DB::ArgumentBuffer<'q>) -> Result<IsNull, BoxDynError> {
        use AnswerValue::*;
        use Attendance::*;
        match self {
            No { veto: true } => "veto".encode_by_ref(buf),
            No { veto: false } => "no".encode_by_ref(buf),
            Yes {
                attendance: Optional,
            } => "yes:optional".encode_by_ref(buf),
            Yes {
                attendance: Required,
            } => "yes:required".encode_by_ref(buf),
        }
    }
}

impl<'r> Decode<'r, Sqlite> for AnswerValue {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        use Attendance::*;
        let text = <&str as Decode<'r, Sqlite>>::decode(value)?;
        match text {
            "no" => Ok(AnswerValue::no(false)),
            "veto" => Ok(AnswerValue::no(true)),
            "yes:optional" => Ok(AnswerValue::yes(Optional)),
            "yes:required" => Ok(AnswerValue::yes(Required)),
            other => Err(format!("Invalid answer value: {other}").into()),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, sqlx::Type, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Attendance {
    Optional,
    /// Admins and people with a special permission can set their attendance to "required".
    /// Participants with required attendance will never be removed from the event
    /// even if the event is over capacity.
    Required,
}

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize, FromFormField)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum DateSelectionStrategy {
    #[field(value = "at_random")]
    AtRandom,
    #[field(value = "to_maximize_participants")]
    ToMaximizeParticipants,
    // TODO: everyone
}

impl fmt::Display for DateSelectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateSelectionStrategy::AtRandom => write!(f, "at random"),
            DateSelectionStrategy::ToMaximizeParticipants => write!(f, "to maximize participants"),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub(crate) struct Location<Id = i64> {
    pub(crate) id: Id,
    pub(crate) nameplate: String,
    pub(crate) street: String,
    pub(crate) street_number: String,
    pub(crate) plz: String,
    pub(crate) city: String,
    #[sqlx(try_from = "i64")]
    pub(crate) floor: i8,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod count_yes_answers {
        use crate::database::New;

        use super::*;

        #[test]
        fn option_with_no_answers_has_zero_yes_answers() {
            let option: PollOption<New> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc().into(),
                answers: vec![],
            };
            assert_eq!(0, option.count_yes_answers());
        }

        #[test]
        fn counts_yes_answers() {
            let option: PollOption<New> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc().into(),
                answers: vec![
                    answer(AnswerValue::yes(Attendance::Optional), UserId(1)),
                    answer(AnswerValue::yes(Attendance::Required), UserId(2)),
                ],
            };
            assert_eq!(2, option.count_yes_answers());
        }

        #[test]
        fn counts_yes_answers_while_ignoring_no_values() {
            let option: PollOption<New> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc().into(),
                answers: vec![
                    answer(AnswerValue::no(false), UserId(1)),
                    answer(AnswerValue::yes(Attendance::Optional), UserId(2)),
                ],
            };
            assert_eq!(1, option.count_yes_answers());
        }

        fn answer(value: AnswerValue, user: UserId) -> Answer<New> {
            Answer {
                value,
                id: (),
                user,
            }
        }
    }
}

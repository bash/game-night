use crate::template::{PageBuilder, PageType};
use crate::users::User;
use rocket::{get, routes, uri, Route};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Database, Decode, Encode, Sqlite, Type};
use time::{Date, OffsetDateTime, Time};

mod new;

pub(crate) fn routes() -> Vec<Route> {
    routes![poll_page, new::new_poll_page, new::new_poll]
}

#[get("/poll")]
fn poll_page(page: PageBuilder<'_>, user: User) -> Template {
    let new_poll_uri = user.can_manage_poll().then(|| uri!(new::new_poll_page()));
    page.type_(PageType::Poll)
        .render("poll", context! { new_poll_uri })
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Poll<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    #[sqlx(try_from = "i64")]
    pub(crate) min_participants: u64,
    #[sqlx(try_from = "i64")]
    pub(crate) max_participants: u64,
    pub(crate) strategy: DateSelectionStrategy,
    pub(crate) description: String,
    pub(crate) open_until: OffsetDateTime,
    pub(crate) closed: bool,
    pub(crate) created_by: UserRef,
    #[sqlx(skip)]
    pub(crate) options: Vec<PollOption<Id, UserRef>>,
}

impl<Id, UserRef> Poll<Id, UserRef> {
    pub(crate) fn materialize(self, user: User, options: Vec<PollOption<Id>>) -> Poll<Id> {
        Poll {
            id: self.id,
            min_participants: self.min_participants,
            max_participants: self.max_participants,
            strategy: self.strategy,
            description: self.description,
            open_until: self.open_until,
            closed: self.closed,
            created_by: user,
            options,
        }
    }
}

impl<Id> Poll<Id> {
    pub(crate) fn state(&self, now: OffsetDateTime) -> PollState {
        if self.closed {
            PollState::Closed
        } else if now > self.open_until {
            PollState::PendingClosure
        } else {
            PollState::Open
        }
    }

    pub(crate) fn is_open(&self, now: OffsetDateTime) -> bool {
        self.state(now).is_open()
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct PollOption<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    pub(crate) date: Date,
    pub(crate) time: Time,
    #[sqlx(skip)]
    pub(crate) answers: Vec<Answer<Id, UserRef>>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Answer<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    pub(crate) value: AnswerValue,
    #[sqlx(rename = "user_id")]
    pub(crate) user: UserRef,
}

impl<Id, UserRef> Answer<Id, UserRef> {
    pub(crate) fn materialize(self, user: User) -> Answer<Id> {
        Answer {
            id: self.id,
            value: self.value,
            user,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub(crate) enum AnswerValue {
    No,
    Yes { attendance: Attendance },
}

impl AnswerValue {
    fn yes(attendance: Attendance) -> Self {
        Self::Yes { attendance }
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
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        use AnswerValue::*;
        use Attendance::*;
        match self {
            No => "no".encode_by_ref(buf),
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
            "no" => Ok(AnswerValue::No),
            "yes:optional" => Ok(AnswerValue::yes(Optional)),
            "yes:required" => Ok(AnswerValue::yes(Required)),
            other => Err(format!("Invalid answer value: {other}").into()),
        }
    }
}

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Attendance {
    Optional,
    /// Admins and people with a special permission can set their attendance to "required".
    /// Participants with required attendance will never be removed from the event
    /// even if the event is over capacity.
    Required,
}

#[derive(Debug, Copy, Clone, sqlx::Type, Serialize)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum DateSelectionStrategy {
    AtRandom,
    ToMaximizeParticipants,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PollState {
    Open,
    PendingClosure,
    Closed,
}

impl PollState {
    pub(crate) fn is_open(self) -> bool {
        matches!(self, PollState::Open)
    }
}

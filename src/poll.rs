use crate::template::{PageBuilder, PageType};
use crate::users::User;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rocket::{get, routes, uri, Route};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Database, Decode, Encode, Sqlite, Type};

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
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    #[sqlx(try_from = "i64")]
    pub(crate) min_participants: u64,
    #[sqlx(try_from = "i64")]
    pub(crate) max_participants: u64,
    pub(crate) strategy: DateSelectionStrategy,
    pub(crate) description: String,
    pub(crate) open_until: NaiveDateTime,
    pub(crate) closed: bool,
    pub(crate) created_by: UserRef,
    #[sqlx(skip)]
    pub(crate) options: Vec<PollOption<Id, UserRef>>,
}

impl<Id> Poll<Id> {
    pub(crate) fn state(&self, now: NaiveDateTime) -> PollState {
        if self.closed {
            PollState::Closed
        } else if now > self.open_until {
            PollState::PendingClosure
        } else {
            PollState::Open
        }
    }

    pub(crate) fn is_open(&self, now: NaiveDateTime) -> bool {
        self.state(now).is_open()
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct PollOption<Id = i64, UserRef = User> {
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) date: NaiveDate,
    pub(crate) time: NaiveTime,
    #[sqlx(skip)]
    pub(crate) votes: Vec<Answer<Id, UserRef>>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Answer<Id = i64, UserRef = User> {
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) value: AnswerValue,
    #[sqlx(rename = "user_id")]
    pub(crate) user: UserRef,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnswerValue {
    No,
    Yes(Attendance),
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
        match self {
            AnswerValue::No => "no".encode_by_ref(buf),
            AnswerValue::Yes(Attendance::Optional) => "yes:optional".encode_by_ref(buf),
            AnswerValue::Yes(Attendance::Required) => "yes:required".encode_by_ref(buf),
        }
    }
}

impl<'r> Decode<'r, Sqlite> for AnswerValue {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let text = <&str as Decode<'r, Sqlite>>::decode(value)?;
        match text {
            "no" => Ok(AnswerValue::No),
            "yes:optional" => Ok(AnswerValue::Yes(Attendance::Optional)),
            "yes:required" => Ok(AnswerValue::Yes(Attendance::Optional)),
            other => Err(format!("Invalid answer value").into()),
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

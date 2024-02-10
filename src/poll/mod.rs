use self::open::open_poll_page;
use crate::database::Repository;
use crate::register::rocket_uri_macro_profile;
use crate::template::PageBuilder;
use crate::users::{User, UserId};
use anyhow::Error;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::response::Debug;
use rocket::{async_trait, get, routes, uri, FromFormField, Request, Route};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Database, Decode, Encode, Sqlite, Type};
use std::{fmt, ops};
use time::OffsetDateTime;

mod finalize;
pub(crate) use finalize::*;
mod email;
use email::PollEmail;
mod new;
mod open;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        poll_page,
        new::new_poll_page,
        new::new_poll,
        new::calendar,
        open::update_answers,
    ]
}

#[get("/", rank = 10)]
async fn poll_page(
    mut repository: Box<dyn Repository>,
    page: PageBuilder<'_>,
    user: User,
) -> Result<Template, Debug<Error>> {
    match repository.get_open_poll().await? {
        Some(poll) => Ok(open_poll_page(
            page,
            poll,
            user,
            repository.get_users().await?,
        )),
        None => Ok(no_open_poll_page(page, user)),
    }
}

fn no_open_poll_page(page: PageBuilder<'_>, user: User) -> Template {
    let new_poll_uri = user.can_manage_poll().then(|| uri!(new::new_poll_page()));
    let profile_uri = uri!(profile());
    page.render("poll", context! { new_poll_uri, profile_uri })
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Poll<Id = i64, UserRef = User, LocationRef = Location> {
    pub(crate) id: Id,
    #[sqlx(try_from = "i64")]
    pub(crate) min_participants: usize,
    #[sqlx(try_from = "i64")]
    pub(crate) max_participants: usize,
    pub(crate) strategy: DateSelectionStrategy,
    pub(crate) description: String,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) open_until: OffsetDateTime,
    pub(crate) closed: bool,
    pub(crate) created_by: UserRef,
    #[sqlx(rename = "location_id")]
    pub(crate) location: LocationRef,
    #[sqlx(skip)]
    pub(crate) options: Vec<PollOption<Id, UserRef>>,
}

impl<Id, UserRef, LocationRef> Poll<Id, UserRef, LocationRef> {
    pub(crate) fn materialize(
        self,
        user: User,
        location: Location,
        options: Vec<PollOption<Id>>,
    ) -> Poll<Id> {
        Poll {
            id: self.id,
            min_participants: self.min_participants,
            max_participants: self.max_participants,
            strategy: self.strategy,
            description: self.description,
            open_until: self.open_until,
            closed: self.closed,
            created_by: user,
            location,
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
pub(crate) struct PollOption<Id = i64, UserRef = User> {
    pub(crate) id: Id,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) starts_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub(crate) ends_at: OffsetDateTime,
    #[sqlx(skip)]
    pub(crate) answers: Vec<Answer<Id, UserRef>>,
}

impl<Id, UserRef> PollOption<Id, UserRef> {
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

#[derive(Debug)]
pub(crate) struct YesAnswer<UserRef = User>(pub(crate) Attendance, pub(crate) UserRef);

impl<Id, UserRef: Clone> Answer<Id, UserRef> {
    pub(crate) fn yes(&self) -> Option<YesAnswer<UserRef>> {
        use AnswerValue::*;
        match self.value {
            No { .. } => None,
            Yes { attendance } => Some(YesAnswer(attendance, self.user.clone())),
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
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
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
}

impl fmt::Display for DateSelectionStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DateSelectionStrategy::AtRandom => write!(f, "at random"),
            DateSelectionStrategy::ToMaximizeParticipants => write!(f, "to maximize participants"),
        }
    }
}

#[derive(Debug, sqlx::FromRow, Serialize)]
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

pub(crate) struct Open<T>(T);

impl<T> ops::Deref for Open<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Open<Poll> {
    type Error = Option<Error>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let mut repository: Box<dyn Repository> = try_outcome!(FromRequest::from_request(request)
            .await
            .map_error(|(s, e)| (s, Some(e))));
        match repository.get_open_poll().await {
            Ok(Some(poll)) => Outcome::Success(Open(poll)),
            Ok(None) => Outcome::Error((Status::BadRequest, None)),
            Err(error) => Outcome::Error((Status::InternalServerError, Some(error))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod count_yes_answers {
        use super::*;

        #[test]
        fn option_with_no_answers_has_zero_yes_answers() {
            let option: PollOption<_, ()> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc(),
                ends_at: OffsetDateTime::now_utc(),
                answers: vec![],
            };
            assert_eq!(0, option.count_yes_answers());
        }

        #[test]
        fn counts_yes_answers() {
            let option: PollOption<_, ()> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc(),
                ends_at: OffsetDateTime::now_utc(),
                answers: vec![
                    answer(AnswerValue::yes(Attendance::Optional)),
                    answer(AnswerValue::yes(Attendance::Required)),
                ],
            };
            assert_eq!(2, option.count_yes_answers());
        }

        #[test]
        fn counts_yes_answers_while_ignoring_no_values() {
            let option: PollOption<_, ()> = PollOption {
                id: (),
                starts_at: OffsetDateTime::now_utc(),
                ends_at: OffsetDateTime::now_utc(),
                answers: vec![
                    answer(AnswerValue::no(false)),
                    answer(AnswerValue::yes(Attendance::Optional)),
                ],
            };
            assert_eq!(1, option.count_yes_answers());
        }

        fn answer(value: AnswerValue) -> Answer<(), ()> {
            Answer {
                value,
                id: (),
                user: (),
            }
        }
    }
}

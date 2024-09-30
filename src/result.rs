use anyhow::Error;
use rocket::http::Status;
use rocket::response::Debug;
use rocket::Responder;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

#[derive(Responder)]
pub(crate) enum HttpError {
    Error(Debug<Error>),
    Status(Status),
}

impl From<Error> for HttpError {
    fn from(value: Error) -> Self {
        HttpError::Error(Debug(value))
    }
}

impl From<Status> for HttpError {
    fn from(value: Status) -> Self {
        HttpError::Status(value)
    }
}

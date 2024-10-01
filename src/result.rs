use crate::responder;
use anyhow::Error;
use rocket::http::Status;
use rocket::response::Debug;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

responder! {
    pub(crate) enum HttpError {
        Error(Debug<Error>),
        Status(Status),
    }
}

impl From<Error> for HttpError {
    fn from(value: Error) -> Self {
        Debug(value).into()
    }
}

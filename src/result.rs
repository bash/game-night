use crate::responder;
use rocket::http::Status;
use rocket::response::Debug;

pub(crate) type HttpResult<T> = Result<T, HttpError>;

responder! {
    pub(crate) enum HttpError {
        Error(Debug<anyhow::Error>),
        Status(Status),
    }
}

macro_rules! impl_from {
    ($($source:ty,)*) => {
        $(impl From<$source> for HttpError {
            fn from(value: $source) -> Self {
                Debug(value.into()).into()
            }
        })*
    };
}

impl_from! {
    anyhow::Error,
    std::fmt::Error,
    std::io::Error,
    askama::Error,
}

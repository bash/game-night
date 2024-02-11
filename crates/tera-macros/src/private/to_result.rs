//! Implements [autoref specialization] so that
//! functions can choose to return `Result<T>` or `T`.
//!
//! [autoref specialization]: https://github.com/dtolnay/case-studies/tree/master/autoref-specialization#realistic-application

#[macro_export]
#[doc(hidden)]
macro_rules! __tera_macros_to_result {
    ($expr:expr, $e:ty) => {{
        #[allow(unused_imports)]
        use $crate::private::{ResultKind, ValueKind};
        match $expr {
            expr => (&expr).kind().to_result::<_, $e>(expr),
        }
    }};
}

pub struct ResultTag;

impl ResultTag {
    pub fn to_result<T, E>(&self, result: Result<T, E>) -> Result<T, E> {
        result
    }
}

pub trait ResultKind {
    fn kind(&self) -> ResultTag {
        ResultTag
    }
}

impl<T, E> ResultKind for Result<T, E> {}

pub struct ValueTag;

impl ValueTag {
    pub fn to_result<T, E>(&self, value: T) -> Result<T, E> {
        Ok(value)
    }
}

pub trait ValueKind {
    fn kind(&self) -> ValueTag {
        ValueTag
    }
}

impl<T> ValueKind for &T {}

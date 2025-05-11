use askama::{FastWritable, NO_VALUES};
use askama_json::askama;
use std::fmt;
use time::OffsetDateTime;

mod date_time;
use crate::users::User;
pub(crate) use date_time::*;

pub(crate) fn is_subscribed(user: &User, _: &dyn askama::Values) -> askama::Result<bool> {
    Ok(user
        .email_subscription
        .is_subscribed(OffsetDateTime::now_utc().date()))
}

pub(crate) fn guillemets<W: FastWritable>(
    input: W,
    _: &dyn askama::Values,
) -> askama::Result<Guillemets<W>> {
    Ok(Guillemets(input))
}

pub(crate) struct Guillemets<T>(T);

impl<T> fmt::Display for Guillemets<T>
where
    T: FastWritable,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_into(f, NO_VALUES).map_err(|_| fmt::Error {})
    }
}

impl<T> FastWritable for Guillemets<T>
where
    T: FastWritable,
{
    fn write_into<W: fmt::Write + ?Sized>(
        &self,
        dest: &mut W,
        values: &dyn askama::Values,
    ) -> askama::Result<()> {
        write!(dest, "«")?;
        self.0.write_into(dest, values)?;
        write!(dest, "»")?;
        Ok(())
    }
}

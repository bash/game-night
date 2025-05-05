use super::convert::OffsetDateTimeLike;
use askama::{FastWritable, NO_VALUES};
use askama_json::askama;
use std::fmt;
use time::format_description::well_known::Iso8601 as Iso8601Format;
use time::format_description::{self, BorrowedFormatItem as FormatItem};
use time::macros::format_description;
use time_tz::{timezones, OffsetDateTimeExt as _};

pub(crate) fn time(
    input: impl OffsetDateTimeLike,
    _: &dyn askama::Values,
    format: impl AsRef<str>,
) -> askama::Result<String> {
    let input = input.into_date_time().to_timezone(timezones::db::CET);
    let formatted = match format.as_ref() {
        "{iso_8601}" => input.format(&Iso8601Format::DEFAULT),
        format => {
            let format = parse_format(format)
                .map_err(|e| askama::Error::custom(format!("Invalid format description: {e}")))?;
            input.format(&format)
        }
    };
    formatted.map_err(|e| askama::Error::custom(format!("Error formatting date {input}: {e}")))
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

fn parse_format(
    format: &str,
) -> Result<Vec<FormatItem<'_>>, time::error::InvalidFormatDescription> {
    const DATE_FORMAT: &[FormatItem] =
        format_description!("[day padding:none].\u{2009}[month repr:long padding:none]");
    const DATE_WITH_YEAR_FORMAT: &[FormatItem] =
        format_description!("[day padding:none].\u{2009}[month repr:long]\u{2009}[year repr:full]");
    const TIME_FORMAT: &[FormatItem] = format_description!("[hour padding:none]:[minute]");
    match format {
        "{time}" => Ok(TIME_FORMAT.to_vec()),
        "{date}" => Ok(DATE_FORMAT.to_vec()),
        "{date_with_year}" => Ok(DATE_WITH_YEAR_FORMAT.to_vec()),
        _ => format_description::parse(format),
    }
}

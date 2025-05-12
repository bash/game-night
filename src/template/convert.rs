use crate::iso_8601::Iso8601;
use time::OffsetDateTime;

pub(crate) trait OffsetDateTimeLike {
    fn into_date_time(self) -> OffsetDateTime;
}

impl OffsetDateTimeLike for OffsetDateTime {
    fn into_date_time(self) -> OffsetDateTime {
        self
    }
}

impl OffsetDateTimeLike for Iso8601<OffsetDateTime> {
    fn into_date_time(self) -> OffsetDateTime {
        self.0
    }
}

impl<T> OffsetDateTimeLike for &T
where
    T: OffsetDateTimeLike + Copy,
{
    fn into_date_time(self) -> OffsetDateTime {
        (*self).into_date_time()
    }
}

use super::{Event, EventLifecycle, Location};
use crate::database::Materialized;

pub(crate) trait EventLike {
    fn details(&self) -> SafeEventDetails;
}

macro_rules! impl_safe_event_details {
    (for <$lt:lifetime> $($field:ident : $ty:ty,)*) => {
        trait EventAnyLifecycle {
            $(fn $field<$lt>(&$lt self) -> $ty;)*
        }

        impl<L: EventLifecycle> EventAnyLifecycle for Event<Materialized, L> {
            $(
                fn $field<$lt>(&$lt self) -> $ty {
                    &self.$field
                }
            )*
        }

        impl<L: EventLifecycle> EventLike for Event<Materialized, L> {
            fn details(&self) -> SafeEventDetails<'_> {
                SafeEventDetails(self)
            }
        }

        impl<$lt> SafeEventDetails<$lt> {
            $(
                pub(crate) fn $field(&self) -> $ty {
                    &self.0.$field()
                }
            )*
        }
    };
}

/// Details about an event that are always safe to access
/// (even while the event is polling).
pub(crate) struct SafeEventDetails<'a>(&'a dyn EventAnyLifecycle);

impl_safe_event_details! {
    for <'a>
    title: &'a str,
    description: &'a str,
    location: &'a Location,
}

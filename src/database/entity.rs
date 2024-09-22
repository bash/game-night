/// The entity has not yet been inserted into the database.
#[derive(Debug, Clone)]
pub(crate) struct New;

/// The entity has been fetched from the database
/// but its relations are not yet materialized.
#[derive(Debug, Clone)]
pub(crate) struct Unmaterialized;

/// A fully materialized entity.
#[derive(Debug, Clone)]
pub(crate) struct Materialized;

#[macro_export]
macro_rules! entity_state {
    ($vis:vis trait $trait:ident { $(type $name:ident $(: $bound:path)? = $new:ty => $unmaterialized:ty => $materialized:ty;)* }) => {
        pub(crate) trait $trait { $(type $name $(: $bound)? ;)* }
        impl $trait for crate::database::New { $(type $name = $new;)* }
        impl $trait for crate::database::Unmaterialized { $(type $name = $unmaterialized;)* }
        impl $trait for crate::database::Materialized { $(type $name = $materialized;)* }
    };
}

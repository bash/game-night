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

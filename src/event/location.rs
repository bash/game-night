use crate::database::Materialized;
use crate::entity_state;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub(crate) struct Location<S: LocationState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) nameplate: String,
    pub(crate) street: String,
    pub(crate) street_number: String,
    pub(crate) plz: String,
    pub(crate) city: String,
    #[sqlx(try_from = "i64")]
    pub(crate) floor: i8,
}

entity_state! {
    pub(crate) trait LocationState {
        type Id = () => i64 => i64;
    }
}

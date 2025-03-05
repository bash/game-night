#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub(crate) struct Location<Id = i64> {
    pub(crate) id: Id,
    pub(crate) nameplate: String,
    pub(crate) street: String,
    pub(crate) street_number: String,
    pub(crate) plz: String,
    pub(crate) city: String,
    #[sqlx(try_from = "i64")]
    pub(crate) floor: i8,
}

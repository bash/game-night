use crate::users::{User, UserId};
use diesel::prelude::*;
use diesel_derive_newtype::DieselNewType;
use std::{fmt, ops};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, sqlx::Type, DieselNewType, rocket::FromForm)]
#[sqlx(transparent)]
#[form(transparent)]
pub(crate) struct LocationId(pub(crate) i64);

impl fmt::Display for LocationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::locations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct RawLocation {
    pub(crate) id: LocationId,
    pub(crate) description: String,
    pub(crate) nameplate: String,
    pub(crate) street: String,
    pub(crate) street_number: String,
    pub(crate) plz: String,
    pub(crate) city: String,
    pub(crate) floor: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct Location {
    pub(crate) location: RawLocation,
    pub(crate) organizers: Vec<Organizer>,
}

impl ops::Deref for Location {
    type Target = RawLocation;

    fn deref(&self) -> &Self::Target {
        &self.location
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, sqlx::Type, DieselNewType)]
#[sqlx(transparent)]
pub(crate) struct OrganizerId(pub(crate) i64);

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(table_name = crate::schema::organizers)]
#[diesel(belongs_to(RawLocation, foreign_key = location_id))]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct RawOrganizer {
    pub(crate) id: OrganizerId,
    pub(crate) location_id: LocationId,
    pub(crate) user_id: UserId,
}

#[derive(Debug, Clone)]
pub(crate) struct Organizer {
    pub(crate) organizer: RawOrganizer,
    pub(crate) user: User,
}

impl ops::Deref for Organizer {
    type Target = RawOrganizer;

    fn deref(&self) -> &Self::Target {
        &self.organizer
    }
}

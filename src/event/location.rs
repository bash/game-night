use crate::database::{Materialized, Unmaterialized};
use crate::entity_state;
use crate::users::{User, UserId};

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Location<S: LocationState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) description: String,
    pub(crate) nameplate: String,
    pub(crate) street: String,
    pub(crate) street_number: String,
    pub(crate) plz: String,
    pub(crate) city: String,
    #[sqlx(try_from = "i64")]
    pub(crate) floor: i8,
    #[sqlx(skip)]
    pub(crate) organizers: S::Organizers,
}

impl Location<Unmaterialized> {
    pub(crate) fn into_materialized(self, organizers: Vec<Organizer>) -> Location {
        Location {
            id: self.id,
            description: self.description,
            nameplate: self.nameplate,
            street: self.street,
            street_number: self.street_number,
            city: self.city,
            floor: self.floor,
            plz: self.plz,
            organizers,
        }
    }
}

entity_state! {
    pub(crate) trait LocationState {
        type Id = () => i64 => i64;
        type Organizers: Default = () => () => Vec<Organizer>;
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Organizer<S: OrganizerState = Materialized> {
    pub(crate) id: S::Id,
    #[sqlx(rename = "user_id")]
    pub(crate) user: S::User,
}

entity_state! {
    pub(crate) trait OrganizerState {
        type Id = () => i64 => i64;
        type User = UserId => UserId => User;
    }
}

impl Organizer<Unmaterialized> {
    pub(crate) fn into_materialized(self, user: User) -> Organizer {
        Organizer { id: self.id, user }
    }
}

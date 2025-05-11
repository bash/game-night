use crate::database::{Materialized, Unmaterialized};
use crate::entity_state;
use crate::users::{User, UserId};

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Group<S: GroupState = Materialized> {
    pub(crate) id: S::Id,
    pub(crate) name: String,
    #[sqlx(skip)]
    pub(crate) members: S::Members,
}

entity_state! {
    pub(crate) trait GroupState {
        type Id = () => i64 => i64;
        type Members: Default = Vec<UserId> => () => Vec<User>;
    }
}

impl Group<Unmaterialized> {
    pub(crate) fn into_materialized(self, members: Vec<User>) -> Group<Materialized> {
        Group {
            id: self.id,
            name: self.name,
            members,
        }
    }
}

impl Group {
    pub(crate) fn has_member(&self, user: &User) -> bool {
        self.members.iter().any(|m| m.id == user.id)
    }
}

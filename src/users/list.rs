use super::{User, UsersQuery};
use crate::auth::{AuthorizedTo, ManageUsers};
use crate::{auto_resolve, HttpResult, PageBuilder, Repository};
use anyhow::Result;
use rocket::get;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[get("/users")]
pub(crate) async fn list_users(
    _guard: AuthorizedTo<ManageUsers>,
    page: PageBuilder<'_>,
    mut users: UsersProvider,
) -> HttpResult<Template> {
    Ok(page.render("users", context! { users: users.active().await? }))
}

auto_resolve! {
    pub(crate) struct UsersProvider {
        query: UsersQuery,
        repository: Box<dyn Repository>,
    }
}

impl UsersProvider {
    async fn active(&mut self) -> Result<Vec<UserViewModel>> {
        let mut users = Vec::new();
        for user in self.query.active().await? {
            users.push(UserViewModel::for_user(user, self.repository.as_mut()).await?);
        }
        Ok(users)
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UserViewModel {
    #[serde(flatten)]
    user: User,
    has_push_subscription: bool,
}

impl UserViewModel {
    async fn for_user(user: User, repository: &mut dyn Repository) -> Result<Self> {
        let has_push_subscription = repository.has_push_subscription(user.id).await?;
        Ok(Self {
            user,
            has_push_subscription,
        })
    }
}

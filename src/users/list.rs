use super::{User, UsersQuery};
use crate::auth::{AuthorizedTo, ManageUsers};
use crate::template_v2::responder::Templated;
use crate::{auto_resolve, HttpResult, PageBuilder, Repository};
use anyhow::Result;
use rocket::get;
use std::ops;
use templates::UsersPage;

#[get("/users")]
pub(crate) async fn list_users(
    _guard: AuthorizedTo<ManageUsers>,
    page: PageBuilder<'_>,
    mut users: UsersProvider,
) -> HttpResult<Templated<UsersPage>> {
    let template = UsersPage {
        users: users.active().await?,
        ctx: page.build(),
    };
    Ok(Templated(template))
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

#[derive(Debug, Clone)]
pub(crate) struct UserViewModel {
    user: User,
    has_push_subscription: bool,
}

impl ops::Deref for UserViewModel {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
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

mod templates {
    use super::UserViewModel;
    use crate::template_v2::prelude::*;
    use crate::users::{Role, UserId};

    #[derive(Template, Debug)]
    #[template(path = "users.html")]
    pub(crate) struct UsersPage {
        pub(super) users: Vec<UserViewModel>,
        pub(super) ctx: PageContext,
    }

    impl UsersPage {
        fn user_by_id(&self, user_id: UserId) -> Option<&UserViewModel> {
            self.users.iter().find(|u| u.id == user_id)
        }
    }

    fn fmt_role(role: Role) -> &'static str {
        match role {
            Role::Admin => "admin",
            Role::Guest => "guest",
        }
    }
}

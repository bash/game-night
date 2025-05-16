use super::UserCommands;
use crate::{auth::LoginState, users::User};
use rocket::{
    async_trait,
    fairing::{Fairing, Info, Kind},
    http::Method,
    trace::warn,
    Data, Request,
};
use time::OffsetDateTime;

pub(crate) struct LastActivity;

#[async_trait]
impl Fairing for LastActivity {
    fn info(&self) -> Info {
        Info {
            name: "Track Last User Activity",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        if req.method() == Method::Post && is_authenticated_and_not_impersonating(req).await {
            if let (Some(user), Some(mut users)) = (
                req.guard::<User>().await.succeeded(),
                req.guard::<UserCommands>().await.succeeded(),
            ) {
                _ = users
                    .update_last_active(user.id, OffsetDateTime::now_utc())
                    .await
                    .inspect_err(|err| {
                        warn!(
                            "Failed to record last active timestamp of user {user_id}: {err:?}",
                            user_id = user.id.0,
                            err = err,
                        )
                    });
            }
        }
    }
}

async fn is_authenticated_and_not_impersonating(req: &mut Request<'_>) -> bool {
    matches!(
        req.guard().await.succeeded(),
        Some(LoginState::Authenticated(_user_id))
    )
}

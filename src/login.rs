use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::users::UserId;
use anyhow::Error;
use chrono::{DateTime, Duration, Local};
use rand::distributions::Alphanumeric;
use rand::Rng;
use rocket::get;
use rocket::http::CookieJar;
use rocket::response::{Debug, Redirect};

#[get("/login?<token>")]
pub(crate) async fn login_with_token<'r>(
    token: &'r str,
    cookies: &'r CookieJar<'r>,
    mut repository: Box<dyn Repository>,
) -> Result<Redirect, Debug<Error>> {
    if let Some(user_id) = repository.use_login_token(token).await? {
        cookies.set_user_id(user_id);
        Ok(Redirect::to("/"))
    } else {
        todo!()
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct LoginToken {
    #[sqlx(rename = "type")]
    pub(crate) type_: LoginTokenType,
    pub(crate) token: String,
    pub(crate) user_id: UserId,
    pub(crate) valid_until: DateTime<Local>,
}

impl LoginToken {
    pub(crate) fn generate_one_time(user_id: UserId) -> Self {
        let one_time_token_expiration = Duration::minutes(30);
        let valid_until = Local::now() + one_time_token_expiration;
        Self {
            type_: LoginTokenType::OneTime,
            token: generate_token(),
            user_id,
            valid_until,
        }
    }

    pub(crate) fn generate_reusable(user_id: UserId, valid_until: DateTime<Local>) -> Self {
        Self {
            type_: LoginTokenType::Reusable,
            token: generate_token(),
            user_id,
            valid_until,
        }
    }
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub(crate) enum LoginTokenType {
    /// Short-lived one time tokens
    OneTime,
    /// Long-lived reusable tokens
    Reusable,
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(20)
        .map(|d| d.to_string())
        .collect()
}

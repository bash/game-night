use super::LoginToken;
use crate::authentication::CookieJarExt;
use crate::database::Repository;
use crate::users::User;
use rocket::fairing::{self, Fairing};
use rocket::http::uri::{Absolute, Origin};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::{async_trait, get, Request};
use url::Url;

pub(crate) fn auto_login_fairing() -> impl Fairing {
    fairing::AdHoc::on_request("Auto-Login", |req, _data| {
        Box::pin(async { _ = auto_login(req).await })
    })
}

pub(crate) fn with_autologin_token(uri: Absolute<'_>, token: &LoginToken) -> String {
    let mut url = Url::parse(&uri.to_string()).unwrap();
    url.query_pairs_mut().append_pair(QUERY_PARAM, &token.token);
    url.to_string()
}

#[get("/_auto-login-redirect")]
pub(super) fn auto_login_redirect(redirect: AutoLoginRedirect) -> Redirect {
    Redirect::to(redirect.0)
}

const QUERY_PARAM: &str = "autologin";

async fn auto_login(request: &mut Request<'_>) {
    if let Some(token) = get_token(request) {
        try_login(request, token).await;

        // This is entire thing is a workaround because fairings can't preemptively
        // complete the request: https://github.com/SergioBenitez/Rocket/issues/749
        request.local_cache(|| AutoLoginRedirect(uri_without_token(request.uri())));
        request.set_uri("/_auto-login-redirect".try_into().unwrap());
    }
}

fn get_token<'a>(request: &'a Request<'_>) -> Option<&'a str> {
    request.query_value(QUERY_PARAM).and_then(|v| v.ok())
}

// TODO: this is extremely ugly, but I can't be bothered to fix it right now
fn uri_without_token(uri: &Origin<'_>) -> String {
    let url = Url::parse(&format!("http://example.com{uri}")).unwrap();
    let query = url.query_pairs().filter(|(name, _)| name != QUERY_PARAM);
    let mut url = url.clone();
    url.query_pairs_mut().clear().extend_pairs(query);

    if let Some(query) = url.query().filter(|q| !q.is_empty()) {
        format!("{}?{}", url.path(), query)
    } else {
        url.path().to_owned()
    }
}

async fn try_login(request: &Request<'_>, token: &str) {
    if let Outcome::Success(mut repository) = request.guard::<Box<dyn Repository>>().await {
        if request.guard::<Option<User>>().await.unwrap().is_none() {
            if let Ok(Some(user_id)) = repository.use_login_token(token).await {
                request.cookies().set_user_id(user_id);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AutoLoginRedirect(String);

#[async_trait]
impl<'r> FromRequest<'r> for AutoLoginRedirect {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(
            request
                .local_cache(|| AutoLoginRedirect("/".to_string()))
                .clone(),
        )
    }
}

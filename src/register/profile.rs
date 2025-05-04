use super::delete::rocket_uri_macro_delete_profile_page;
use super::AstronomicalSymbol;
use crate::database::Repository;
use crate::push::PushEndpoints;
use crate::template::PageBuilder;
use crate::template_v2::responder::Templated;
use crate::users::{rocket_uri_macro_list_users, EmailSubscription, ASTRONOMICAL_SYMBOLS};
use crate::users::{User, UserPatch};
use anyhow::{Error, Result};
use rocket::form::Form;
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri, FromForm, State};
use templates::ProfilePage;
use time::Date;

#[get("/profile")]
pub(crate) fn profile(
    page: PageBuilder,
    user: User,
    push_endpoints: &State<PushEndpoints>,
) -> Templated<ProfilePage> {
    let template = ProfilePage {
        can_update_name: user.can_update_name(),
        list_users_uri: user.can_manage_users().then(|| uri!(list_users())),
        delete_profile_uri: uri!(delete_profile_page()),
        push_self_test_uri: uri!(crate::push::self_test),
        symbols: user.can_update_symbol().then_some(ASTRONOMICAL_SYMBOLS),
        push_endpoints: push_endpoints.inner().clone(),
        user,
        ctx: page.build(),
    };
    Templated(template)
}

#[post("/profile", data = "<form>")]
pub(super) async fn update_profile(
    mut repository: Box<dyn Repository>,
    form: Form<UpdateUserForm>,
    user: User,
) -> Result<Redirect, Debug<Error>> {
    let patch = form.into_inner().into_user_patch(&user);
    repository.update_user(user.id, patch).await?;
    Ok(Redirect::to(uri!(profile)))
}

#[derive(Debug, FromForm)]
pub(crate) struct UpdateUserForm {
    #[form(validate = len(1..))]
    name: Option<String>,
    symbol: Option<AstronomicalSymbol>,
    subscribe: bool,
    until: Option<Date>,
}

impl UpdateUserForm {
    fn into_user_patch(self, user: &User) -> UserPatch {
        let name = self
            .name
            .filter(|_| user.can_update_name())
            .map(|name| name.trim().to_owned());
        let email_subscription = Some(to_email_subscription(self.subscribe, self.until));
        let symbol = self.symbol.filter(|_| user.can_update_symbol());
        UserPatch {
            name,
            email_subscription,
            symbol,
        }
    }
}

fn to_email_subscription(subscribe: bool, until: Option<Date>) -> EmailSubscription {
    match (subscribe, until) {
        (true, _) => EmailSubscription::Subscribed,
        (false, Some(until)) => EmailSubscription::TemporarilyUnsubscribed {
            until: until.into(),
        },
        (false, None) => EmailSubscription::PermanentlyUnsubscribed,
    }
}

mod templates {
    use crate::push::PushEndpoints;
    use crate::template_v2::prelude::*;
    use crate::users::{AstronomicalSymbol, EmailSubscription, User};
    use rocket::http::uri::Origin;
    use serde_json::json;

    #[derive(Template, Debug)]
    #[template(path = "register/profile.html")]
    pub(crate) struct ProfilePage {
        pub(crate) can_update_name: bool,
        pub(crate) list_users_uri: Option<Origin<'static>>,
        pub(crate) delete_profile_uri: Origin<'static>,
        pub(crate) push_self_test_uri: Origin<'static>,
        pub(crate) symbols: Option<&'static [AstronomicalSymbol]>,
        pub(crate) push_endpoints: PushEndpoints,
        pub(crate) user: User,
        pub(crate) ctx: PageContext,
    }

    impl ProfilePage {
        fn subscription_values(&self) -> serde_json::Value {
            use EmailSubscription::*;
            let permanence = match self.user.email_subscription {
                PermanentlyUnsubscribed => "permanent",
                Subscribed | TemporarilyUnsubscribed { .. } => "temporary",
            };
            let subscription = match self.user.email_subscription {
                Subscribed => "on",
                TemporarilyUnsubscribed { .. } | PermanentlyUnsubscribed => "off",
            };
            let until = match self.user.email_subscription {
                TemporarilyUnsubscribed { until } => Some(serde_json::to_value(until).unwrap()),
                Subscribed | PermanentlyUnsubscribed => None,
            };
            json!({
                "permanence": permanence,
                "subscription": subscription,
                "until": until
            })
        }
    }
}

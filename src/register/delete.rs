use crate::database::Repository;
use crate::decorations::Random;
use crate::invitation::{Invitation, Passphrase};
use crate::login::{Logout, RedirectUri};
use crate::template::PageBuilder;
use crate::template_v2::responder::Templated;
use crate::users::User;
use crate::HttpResult;
use rand::rng;
use rocket::response::Responder;
use rocket::{get, post, uri};
use templates::{DeleteProfilePage, ProfileDeletedPage};
use time::{Duration, OffsetDateTime};

#[get("/profile/delete")]
pub(crate) fn delete_profile_page(page: PageBuilder, user: User) -> impl Responder {
    Templated(DeleteProfilePage {
        user,
        random: Random::default(),
        ctx: page.build(),
    })
}

#[post("/profile/delete")]
pub(crate) async fn delete_profile(
    mut repository: Box<dyn Repository>,
    user: User,
) -> HttpResult<Logout> {
    let invitation = repository.add_invitation(goodbye_invitation(&user)).await?;
    repository.delete_user(user.id).await?;
    let redirect_uri = RedirectUri(uri!(profile_deleted_page(user.name, invitation.passphrase)));
    Ok(Logout(redirect_uri))
}

fn goodbye_invitation(user: &User) -> Invitation<()> {
    let valid_until = OffsetDateTime::now_utc() + Duration::days(365);
    Invitation::builder()
        .role(user.role)
        .valid_until(valid_until)
        .comment(format!("Goodbye invitation for '{}'", user.name))
        .build(&mut rng())
}

#[get("/profile/deleted?<name>&<passphrase>")]
pub(crate) fn profile_deleted_page(
    page: PageBuilder,
    name: String,
    passphrase: Passphrase,
) -> impl Responder {
    Templated(ProfileDeletedPage {
        name,
        passphrase,
        random: Random::default(),
        ctx: page.build(),
    })
}

mod templates {
    use crate::decorations::Random;
    use crate::invitation::Passphrase;
    use crate::template_v2::prelude::*;
    use crate::users::User;

    #[derive(Template, Debug)]
    #[template(path = "register/delete.html")]
    pub(crate) struct DeleteProfilePage {
        pub(crate) user: User,
        pub(crate) random: Random,
        pub(crate) ctx: PageContext,
    }

    #[derive(Template, Debug)]
    #[template(path = "register/deleted.html")]
    pub(crate) struct ProfileDeletedPage {
        pub(crate) name: String,
        pub(crate) passphrase: Passphrase,
        pub(crate) random: Random,
        pub(crate) ctx: PageContext,
    }
}

use crate::auth::{AuthorizedTo, Invite};
use crate::database::Repository;
use crate::register::rocket_uri_macro_register_page;
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::uri;
use crate::uri::UriBuilder;
use crate::users::models::{NewUser, UserV2};
use crate::users::{AstronomicalSymbol, EmailSubscription, Role, User, UserId, UserQueries};
use anyhow::Result;
use itertools::Itertools as _;
use rand::{prelude::*, rng};
use rocket::form::Form;
use rocket::http::uri::Absolute;
use rocket::yansi::Paint as _;
use rocket::{get, post, routes, FromForm, FromFormField, Route};
use std::num::NonZeroU32;
use time::{Duration, OffsetDateTime};

mod wordlist;
pub(crate) use self::wordlist::*;
mod passphrase;
pub(crate) use self::passphrase::*;

pub(crate) fn routes() -> Vec<Route> {
    routes![invite_page, generate_invitation]
}

#[get("/invite")]
async fn invite_page(
    user: AuthorizedTo<Invite>,
    mut users: UserQueries,
    page: PageContextBuilder<'_>,
) -> HttpResult<Templated<InvitePage>> {
    let users = users.active().await?;
    let user = user.into_inner();
    Ok(Templated(InvitePage {
        users,
        user,
        ctx: page.build(),
    }))
}

#[derive(Template, Debug)]
#[template(path = "invitation/invite.html")]
struct InvitePage {
    users: Vec<UserV2>,
    user: User,
    ctx: PageContext,
}

#[post("/invite", data = "<form>")]
async fn generate_invitation(
    _user: AuthorizedTo<Invite>,
    page: PageContextBuilder<'_>,
    mut repository: Box<dyn Repository>,
    form: Form<GenerateInvitationData>,
    uri_builder: UriBuilder,
) -> HttpResult<Templated<InvitationPage>> {
    let form = form.into_inner();
    let lifetime = Duration::days(i64::from(u32::from(form.lifetime_in_days)));
    let valid_until = OffsetDateTime::now_utc() + lifetime;
    let invitation = Invitation::builder()
        .role(Role::Guest)
        .created_by(form.inviter)
        .valid_until(valid_until)
        .comment(&form.comment)
        .build(&mut rng());
    let invitation = repository.add_invitation(invitation).await?;
    let passphrase = invitation.passphrase.clone();
    let register_uri = uri!(
        uri_builder,
        register_page(passphrase = Some(invitation.passphrase))
    );
    Ok(Templated(InvitationPage {
        passphrase,
        register_uri,
        form,
        ctx: page.build(),
    }))
}

#[derive(Template, Debug)]
#[template(path = "invitation/invitation.html")]
struct InvitationPage {
    passphrase: Passphrase,
    register_uri: Absolute<'static>,
    form: GenerateInvitationData,
    ctx: PageContext,
}

#[derive(Debug, FromForm)]
struct GenerateInvitationData {
    lifetime_in_days: NonZeroU32,
    inviter: UserId,
    comment: String,
}

#[derive(Debug, Clone, Copy, FromFormField)]
enum InvitationLifetime {
    Short,
    Long,
}

impl From<InvitationLifetime> for Duration {
    fn from(value: InvitationLifetime) -> Self {
        use InvitationLifetime::*;
        match value {
            Short => Duration::days(30),
            Long => Duration::days(365),
        }
    }
}

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(transparent)]
pub(crate) struct InvitationId(pub(crate) i64);

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Invitation<Id = InvitationId> {
    pub(crate) id: Id,
    pub(crate) role: Role,
    pub(crate) created_by: Option<UserId>,
    pub(crate) passphrase: Passphrase,
    pub(crate) comment: String,
    pub(crate) valid_until: Option<OffsetDateTime>,
    pub(crate) used_by: Option<UserId>,
}

#[derive(Debug, Default)]
pub(crate) struct InvitationBuilder {
    role: Role,
    created_by: Option<UserId>,
    valid_until: Option<OffsetDateTime>,
    comment: String,
}

impl InvitationBuilder {
    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    pub(crate) fn created_by(mut self, user_id: impl Into<Option<UserId>>) -> Self {
        self.created_by = user_id.into();
        self
    }

    pub(crate) fn valid_until(mut self, valid_until: impl Into<Option<OffsetDateTime>>) -> Self {
        self.valid_until = valid_until.into();
        self
    }

    pub(crate) fn comment(mut self, comment: impl ToString) -> Self {
        self.comment = comment.to_string();
        self
    }

    pub(crate) fn build<R: Rng>(self, rng: &mut R) -> Invitation<()> {
        Invitation {
            id: (),
            role: self.role,
            created_by: self.created_by,
            valid_until: self.valid_until,
            used_by: None,
            passphrase: rng.random(),
            comment: self.comment,
        }
    }
}

impl Invitation<()> {
    pub(crate) fn builder() -> InvitationBuilder {
        InvitationBuilder::default()
    }

    pub(crate) fn with_id(self, id: InvitationId) -> Invitation {
        Invitation {
            id,
            role: self.role,
            created_by: self.created_by,
            passphrase: self.passphrase,
            comment: self.comment,
            used_by: self.used_by,
            valid_until: self.valid_until,
        }
    }
}

impl<Id> Invitation<Id> {
    pub(crate) fn to_user(
        &self,
        name: String,
        symbol: AstronomicalSymbol,
        email_address: String,
        campaign: Option<String>,
    ) -> NewUser {
        NewUser {
            name,
            symbol,
            email_address,
            email_subscription: EmailSubscription::default(),
            role: self.role,
            invited_by: self.created_by,
            campaign,
        }
    }
}

pub(crate) async fn invite_admin_user(
    users: &mut UserQueries,
    repository: &mut dyn Repository,
) -> Result<()> {
    if !users.has().await? {
        let invitation = get_or_create_invitation(repository).await?;
        eprintln!("👑 {}", "admin".magenta().bold());
        eprintln!("   >> {}: {}", "invitation".blue(), &invitation.passphrase);
    }

    Ok(())
}

async fn get_or_create_invitation(repository: &mut dyn Repository) -> Result<Invitation> {
    Ok(match repository.get_admin_invitation().await? {
        Some(invitation) => invitation,
        None => {
            let invitation = InvitationBuilder::default()
                .role(Role::Admin)
                .comment("auto-generated admin invite")
                .build(&mut rng());
            repository.add_invitation(invitation).await?
        }
    })
}

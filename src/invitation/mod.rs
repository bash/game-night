use crate::auth::{AuthorizedTo, Invite};
use crate::database::Repository;
use crate::register::rocket_uri_macro_register_page;
use crate::template::PageBuilder;
use crate::uri::UriBuilder;
use crate::users::{AstronomicalSymbol, EmailSubscription, Role, User, UserId, UsersQuery};
use crate::{uri, HttpResult};
use anyhow::Result;
use rand::{prelude::*, rng};
use rocket::form::Form;
use rocket::yansi::Paint as _;
use rocket::{get, post, routes, FromForm, FromFormField, Route};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
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
    _user: AuthorizedTo<Invite>,
    mut users_query: UsersQuery,
    page: PageBuilder<'_>,
) -> HttpResult<Template> {
    Ok(page.render(
        "invite",
        context! {
            users: users_query.active().await?
        },
    ))
}

#[post("/invite", data = "<form>")]
async fn generate_invitation(
    _user: AuthorizedTo<Invite>,
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
    form: Form<GenerateInvitationData>,
    uri_builder: UriBuilder,
) -> HttpResult<Template> {
    let lifetime = Duration::days(i64::from(u32::from(form.lifetime_in_days)));
    let valid_until = OffsetDateTime::now_utc() + lifetime;
    let invitation = Invitation::builder()
        .role(Role::Guest)
        .created_by(form.inviter)
        .valid_until(valid_until)
        .comment(&form.comment)
        .build(&mut rng());
    let invitation = repository.add_invitation(invitation).await?;
    Ok(page.render(
        "invitation",
        context! {
            passphrase: invitation.passphrase.clone(),
            form: form.into_inner(),
            register_uri: uri!(uri_builder, register_page(passphrase = Some(invitation.passphrase))),
        },
    ))
}

#[derive(Debug, FromForm, Serialize)]
struct GenerateInvitationData {
    lifetime_in_days: NonZeroU32,
    inviter: UserId,
    comment: String,
}

#[derive(Debug, Clone, Copy, FromFormField, Serialize)]
#[serde(rename_all = "snake_case")]
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
    ) -> User<()> {
        User {
            id: (),
            name,
            symbol,
            email_address,
            email_subscription: EmailSubscription::default(),
            role: self.role,
            invited_by: self.created_by,
            campaign,
            can_update_name: true,
            can_answer_strongly: false,
            can_update_symbol: true,
            last_active_at: OffsetDateTime::now_utc().into(),
        }
    }
}

pub(crate) async fn invite_admin_user(repository: &mut dyn Repository) -> Result<()> {
    if !repository.has_users().await? {
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

use crate::database::Repository;
use crate::template::{PageBuilder, PageType};
use crate::users::{AuthorizedTo, Invite, Role, User, UserId};
use anyhow::{Error, Result};
use chrono::{DateTime, Duration, Local};
use rand::prelude::*;
use rocket::form::Form;
use rocket::log::PaintExt as _;
use rocket::response::Debug;
use rocket::{get, launch_meta, launch_meta_, post, routes, uri, FromForm, FromFormField, Route};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Database, Decode, Encode, Sqlite};
use std::fmt;
use yansi::Paint as _;

mod wordlist;
pub(crate) use self::wordlist::*;
mod batch;

pub(crate) fn routes() -> Vec<Route> {
    routes![
        invite_page,
        generate_invitation,
        batch::invite,
        batch::cards
    ]
}

#[get("/invite")]
fn invite_page(page: PageBuilder<'_>, user: Option<User>) -> Template {
    let can_invite = user.map(|u| u.can_invite()).unwrap_or_default();
    page.type_(PageType::Invite).render(
        "invite",
        context! { can_invite, batch_invite_uri: uri!(batch::invite) },
    )
}

#[post("/invite", data = "<form>")]
async fn generate_invitation(
    page: PageBuilder<'_>,
    mut repository: Box<dyn Repository>,
    form: Form<GenerateInvitationData>,
    user: AuthorizedTo<Invite>,
) -> Result<Template, Debug<Error>> {
    let lifetime: Duration = form.lifetime.into();
    let valid_until = Local::now() + lifetime;
    let invitation = Invitation::generate(Role::Guest, Some(user.id), Some(valid_until));
    let invitation = repository.add_invitation(invitation).await?;

    Ok(page.type_(PageType::Invite).render(
        "invitation",
        context! {
            passphrase: invitation.passphrase,
            lifetime: form.lifetime
        },
    ))
}

#[derive(Debug, FromForm)]
struct GenerateInvitationData {
    lifetime: InvitationLifetime,
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
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) role: Role,
    pub(crate) created_by: Option<UserId>,
    pub(crate) passphrase: Passphrase,
    pub(crate) valid_until: Option<DateTime<Local>>,
}

impl Invitation<()> {
    pub(crate) fn with_id(self, id: InvitationId) -> Invitation {
        Invitation {
            id,
            role: self.role,
            created_by: self.created_by,
            passphrase: self.passphrase,
            valid_until: self.valid_until,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub(crate) struct Passphrase(pub(crate) Vec<String>);

impl fmt::Display for Passphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join(" "))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Passphrase
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as HasValueRef<'r>>::ValueRef,
    ) -> Result<Passphrase, Box<dyn std::error::Error + 'static + Send + Sync>> {
        Ok(Self(
            <&str as Decode<DB>>::decode(value)?
                .split(' ')
                .map(ToOwned::to_owned)
                .collect(),
        ))
    }
}

impl<'q> Encode<'q, Sqlite> for Passphrase
where
    &'q str: Encode<'q, Sqlite>,
{
    fn encode_by_ref(&self, buf: &mut <Sqlite as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
        buf.push(SqliteArgumentValue::Text(self.0.join(" ").into()));
        IsNull::No
    }
}

impl sqlx::Type<Sqlite> for Passphrase {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <String as sqlx::Type<Sqlite>>::type_info()
    }
}

impl Invitation<()> {
    pub(crate) fn generate(
        role: Role,
        created_by: Option<UserId>,
        valid_until: Option<DateTime<Local>>,
    ) -> Self {
        Invitation {
            id: (),
            role,
            created_by,
            valid_until,
            passphrase: generate_passphrase(),
        }
    }
}

impl<Id> Invitation<Id> {
    pub(crate) fn to_user(
        &self,
        name: String,
        email_address: String,
        campaign: Option<String>,
    ) -> User<()> {
        User {
            id: (),
            name,
            email_address,
            role: self.role,
            invited_by: self.created_by,
            campaign,
        }
    }
}

pub(crate) async fn invite_admin_user(repository: &mut dyn Repository) -> Result<()> {
    if !repository.has_users().await? {
        launch_meta!("{}{}:", "👑 ".emoji(), "Admin".magenta());
        let invitation = get_or_create_invitation(repository).await?;
        launch_meta_!("invitation: {}", &invitation.passphrase);
    }

    Ok(())
}

async fn get_or_create_invitation(repository: &mut dyn Repository) -> Result<Invitation> {
    Ok(match repository.get_admin_invitation().await? {
        Some(invitation) => invitation,
        None => {
            repository
                .add_invitation(Invitation::generate(Role::Admin, None, None))
                .await?
        }
    })
}

fn generate_passphrase() -> Passphrase {
    let words: Vec<_> = TAUS_WORDLIST
        .choose_multiple(&mut thread_rng(), 4)
        .map(|s| (*s).to_owned())
        .collect();
    Passphrase(words)
}

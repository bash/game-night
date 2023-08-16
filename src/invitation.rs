use crate::database::Repository;
use crate::users::{Role, User, UserId};
use anyhow::Result;
use diceware_wordlists::EFF_LONG_WORDLIST;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rocket::log::PaintExt;
use rocket::{launch_meta, launch_meta_};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Database, Decode, Encode, Sqlite};
use std::error::Error;
use std::fmt;
use yansi::Paint;

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
}

impl Invitation<()> {
    pub(crate) fn with_id(self, id: InvitationId) -> Invitation {
        Invitation {
            id,
            role: self.role,
            created_by: self.created_by,
            passphrase: self.passphrase,
        }
    }
}

#[derive(Debug, Clone)]
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
    ) -> Result<Passphrase, Box<dyn Error + 'static + Send + Sync>> {
        Ok(Self(
            <&str as Decode<DB>>::decode(value)?
                .split(" ")
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
    pub(crate) fn generate(role: Role, created_by: Option<UserId>) -> Self {
        Invitation {
            id: (),
            role,
            created_by,
            passphrase: generate_passphrase(),
        }
    }
}

impl<Id> Invitation<Id> {
    pub(crate) fn to_user(&self, name: String, email_address: String) -> User<()> {
        User {
            id: (),
            name,
            email_address,
            role: self.role,
            invited_by: self.created_by,
        }
    }
}

pub(crate) async fn invite_admin_user(repository: &mut dyn Repository) -> Result<()> {
    launch_meta!(
        "{}{}:",
        <Paint<&str> as PaintExt>::emoji("👑 "),
        Paint::magenta("Admin")
    );

    if !repository.has_users().await? {
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
                .add_invitation(Invitation::generate(Role::Admin, None))
                .await?
        }
    })
}

fn generate_passphrase() -> Passphrase {
    let words: Vec<_> = EFF_LONG_WORDLIST
        .choose_multiple(&mut thread_rng(), 4)
        .map(|s| (*s).to_owned())
        .collect();
    Passphrase(words)
}

use crate::database::Repository;
use crate::users::{Role, User, UserId};
use diceware_wordlists::EFF_LONG_WORDLIST;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rocket::log::PaintExt;
use rocket::{launch_meta, launch_meta_};
use std::error::Error;
use yansi::Paint;

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(transparent)]
pub(crate) struct InvitationId(i64);

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct Invitation<Id = InvitationId> {
    #[sqlx(rename = "rowid")]
    pub(crate) id: Id,
    pub(crate) role: Role,
    pub(crate) created_by: Option<UserId>,
    pub(crate) passphrase: String,
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

pub(crate) async fn invite_admin_user(
    repository: &mut (dyn Repository + Send),
) -> Result<(), Box<dyn Error>> {
    launch_meta!(
        "{}{}:",
        <Paint<&str> as PaintExt>::emoji("👑 "),
        Paint::magenta("Admin")
    );

    if !repository.has_users().await? {
        let invitation = Invitation::generate(Role::Admin, None);
        launch_meta_!("invitation: {}", &invitation.passphrase);
        repository.add_invitation(invitation).await?;
    }

    Ok(())
}

fn generate_passphrase() -> String {
    let words: Vec<_> = EFF_LONG_WORDLIST
        .choose_multiple(&mut thread_rng(), 4)
        .cloned()
        .collect();
    words.join(" ")
}

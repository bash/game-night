use super::Passphrases;
use super::{Invitation, InvitationLifetime};
use crate::auth::{AuthorizedTo, Invite};
use crate::database::Repository;
use crate::users::{Role, User};
use anyhow::{Error, Result};
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};
use time::{Duration, OffsetDateTime};

const INVITATIONS_PER_SHEET: usize = 20;
const INVITATIONS_PER_PAGE: usize = 2;

#[post("/invite/batch")]
pub(super) async fn invite(
    user: AuthorizedTo<Invite>,
    mut repository: Box<dyn Repository>,
) -> Result<Redirect, Debug<Error>> {
    let now = OffsetDateTime::now_utc();
    let invitations = save_invitations(
        repository.as_mut(),
        (0..INVITATIONS_PER_SHEET).map(|_| generate_invitation(&user, now)),
    )
    .await?;
    let passphrases = Passphrases::from_invitations(invitations.iter());
    Ok(Redirect::to(uri!(cards(passphrases))))
}

#[get("/invite/cards?<passphrases>")]
pub(super) async fn cards(passphrases: Passphrases, _user: AuthorizedTo<Invite>) -> Template {
    let pages: Vec<_> = passphrases.0.chunks(INVITATIONS_PER_PAGE).collect();
    Template::render("cards", context! { pages })
}

async fn save_invitations(
    repository: &mut dyn Repository,
    invitations: impl Iterator<Item = Invitation<()>>,
) -> Result<Vec<Invitation>> {
    let mut result = Vec::new();
    for invitation in invitations {
        result.push(repository.add_invitation(invitation).await?);
    }
    Ok(result)
}

fn generate_invitation(user: &User, now: OffsetDateTime) -> Invitation<()> {
    let valid_until = now + Duration::from(InvitationLifetime::Long);
    Invitation::generate(Role::Guest, Some(user.id), Some(valid_until))
}

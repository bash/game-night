use super::{Invitation, InvitationLifetime, Passphrase};
use crate::authorization::{AuthorizedTo, Invite};
use crate::database::Repository;
use crate::users::{Role, User};
use anyhow::{Error, Result};
use chrono::{Duration, Local};
use rocket::form::FromFormField;
use rocket::http::impl_from_uri_param_identity;
use rocket::http::uri::fmt::{Query, UriDisplay};
use rocket::response::{Debug, Redirect};
use rocket::{get, post, uri};
use rocket_dyn_templates::{context, Template};
use std::fmt;

const INVITATIONS_PER_SHEET: usize = 10;
const INVITATIONS_PER_PAGE: usize = INVITATIONS_PER_SHEET / 2;

#[post("/invite/batch")]
pub(super) async fn invite(
    user: AuthorizedTo<Invite>,
    mut repository: Box<dyn Repository>,
) -> Result<Redirect, Debug<Error>> {
    let invitations = save_invitations(
        repository.as_mut(),
        (0..INVITATIONS_PER_SHEET).map(|_| generate_invitation(&user)),
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

fn generate_invitation(user: &User) -> Invitation<()> {
    let valid_until = Local::now() + Duration::from(InvitationLifetime::Long);
    Invitation::generate(Role::Guest, Some(user.id), Some(valid_until))
}

#[derive(Debug)]
pub(super) struct Passphrases(Vec<Passphrase>);

impl Passphrases {
    fn from_invitations<'a>(iter: impl Iterator<Item = &'a Invitation>) -> Self {
        Self(iter.map(|i| i.passphrase.clone()).collect())
    }
}

impl<'v> FromFormField<'v> for Passphrases {
    fn from_value(field: rocket::form::ValueField<'v>) -> rocket::form::Result<'v, Self> {
        Ok(Passphrases(
            field
                .value
                .split(',')
                .map(|p| Passphrase(p.split('-').map(ToOwned::to_owned).collect()))
                .collect(),
        ))
    }
}

impl_from_uri_param_identity!([Query] Passphrases);

impl UriDisplay<Query> for Passphrases {
    fn fmt(&self, f: &mut rocket::http::uri::fmt::Formatter<'_, Query>) -> fmt::Result {
        let value = self
            .0
            .iter()
            .map(|p| p.0.join("-"))
            .collect::<Vec<_>>()
            .join(",");
        f.write_value(value)
    }
}

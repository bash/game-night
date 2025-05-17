use super::{Role, UserQueries};
use crate::auto_resolve;
use crate::invitation::{Invitation, InvitationCommands, InvitationQueries, NewInvitationBuilder};
use anyhow::{Context as _, Result};
use rand::rng;
use rocket::fairing::{self, Fairing};
use rocket::yansi::Paint as _;
use rocket::{error, Orbit, Rocket};

pub(crate) fn invite_admin_user_fairing() -> impl Fairing {
    fairing::AdHoc::on_liftoff("Invite Admin User", |rocket| {
        Box::pin(async move {
            if let Err(e) = try_invite_admin_user(rocket).await {
                error!("{:?}", e);
            }
        })
    })
}

async fn try_invite_admin_user(rocket: &Rocket<Orbit>) -> Result<()> {
    use crate::services::RocketResolveExt as _;
    let mut inviter: AdminUserInviter = rocket.resolve().await?;
    inviter
        .invite_when_needed()
        .await
        .context("failed to invite admin user")
}

auto_resolve! {
    struct AdminUserInviter {
        users: UserQueries,
        invitations: InvitationQueries,
        invitation_cmds: InvitationCommands,
    }
}

impl AdminUserInviter {
    async fn invite_when_needed(&mut self) -> Result<()> {
        if !self.users.has().await? {
            let invitation = self.invite().await?;
            eprintln!("👑 {}", "admin".magenta().bold());
            eprintln!("   >> {}: {}", "invitation".blue(), &invitation.passphrase);
        }
        Ok(())
    }

    async fn invite(&mut self) -> Result<Invitation> {
        match self.invitations.admin().await? {
            Some(invitation) => Ok(invitation),
            None => {
                let invitation = NewInvitationBuilder::default()
                    .role(Role::Admin)
                    .comment("auto-generated admin invite")
                    .build(&mut rng());
                self.invitation_cmds.add(invitation).await
            }
        }
    }
}

use super::LoginToken;
use crate::database::Repository;
use crate::decorations::Random;
use crate::email::{EmailMessage, EmailTemplateContext};
use crate::users::{User, UserMailboxExt as _, UserQueries};
use crate::{auto_resolve, email_template};
use anyhow::Result;
use lettre::message::Mailbox;
use rand::rng;

auto_resolve! {
    pub(super) struct LoginEmailProvider {
        users: UserQueries,
        repository: Box<dyn Repository>,
        email_ctx: EmailTemplateContext,
    }
}

impl LoginEmailProvider {
    pub(super) async fn for_address(
        &mut self,
        email_address: &str,
    ) -> Result<Option<(Mailbox, LoginEmail)>> {
        let Some(user) = self.users.by_email(email_address).await? else {
            return Ok(None);
        };
        self.for_user(user).await.map(Some)
    }

    async fn for_user(&mut self, user: User) -> Result<(Mailbox, LoginEmail)> {
        let token = LoginToken::generate_one_time(user.id, &mut rng());
        self.repository.add_login_token(&token).await?;

        let email = LoginEmail {
            name: user.name.clone(),
            code: token.token,
            random: Random::default(),
            ctx: self.email_ctx.clone(),
        };

        Ok((user.mailbox()?, email))
    }
}

email_template! {
    #[template(html_path = "emails/login.html", txt_path = "emails/login.txt")]
    #[derive(Debug)]
    pub(super) struct LoginEmail {
        name: String,
        code: String,
        random: Random,
        ctx: EmailTemplateContext,
    }
}

impl EmailMessage for LoginEmail {
    fn subject(&self) -> String {
        "Let's Get You Logged In".to_owned()
    }
}

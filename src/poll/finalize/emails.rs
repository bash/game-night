use super::FinalizeContext;
use super::FinalizeResult::{self, *};
use crate::email::EmailMessage;
use crate::login::{with_autologin_token, LoginToken};
use crate::play::rocket_uri_macro_play_page;
use crate::users::{User, UserId};
use anyhow::Result;
use rocket::uri;
use serde::Serialize;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

type Event = crate::event::Event<(), UserId, i64>;

pub(super) async fn send_notification_emails(
    ctx: &mut FinalizeContext,
    result: &FinalizeResult,
) -> Result<()> {
    if let Success(event, invited, _) = result {
        for user in invited {
            send_invited_email(ctx, event, user).await?;
        }
    }
    Ok(())
}

async fn send_invited_email(ctx: &mut FinalizeContext, event: &Event, user: &User) -> Result<()> {
    let event_url = event_url(ctx, user, event).await?;
    let email = InvitedEmail {
        event_datetime: event.starts_at,
        name: &user.name,
        event_url,
    };
    ctx.email_sender.send(user.mailbox()?, &email).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct InvitedEmail<'a> {
    #[serde(serialize_with = "crate::serde_formats::serialize_as_cet")]
    event_datetime: OffsetDateTime,
    name: &'a str,
    event_url: String,
}

impl<'a> EmailMessage for InvitedEmail<'a> {
    fn subject(&self) -> String {
        const FORMAT: &[FormatItem<'_>] =
            format_description!("[day padding:none]. [month repr:long]");
        format!(
            "You're invited to Tau's Game Night on {date}!",
            date = self.event_datetime.format(FORMAT).unwrap()
        )
    }

    fn template_name(&self) -> String {
        "event/invited".to_string()
    }
}

async fn event_url(ctx: &mut FinalizeContext, user: &User, event: &Event) -> Result<String> {
    let token = LoginToken::generate_reusable(user.id, event.ends_at);
    ctx.repository.add_login_token(&token).await?;
    Ok(with_autologin_token(
        uri!(ctx.url_prefix.0.clone(), play_page()),
        &token,
    ))
}

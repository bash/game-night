use super::FinalizeContext;
use crate::email::EmailMessage;
use crate::event::Event;
use crate::login::{with_autologin_token, LoginToken};
use crate::play::rocket_uri_macro_play_page;
use crate::users::User;
use anyhow::Result;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, SinglePart};
use rocket::uri;
use serde::Serialize;
use time::format_description::FormatItem;
use time::macros::format_description;

pub(super) async fn send_notification_emails(
    ctx: &mut FinalizeContext,
    event: &Event,
    invited: &[User],
) -> Result<()> {
    for user in invited {
        send_invited_email(ctx, event, user).await?;
    }
    Ok(())
}

async fn send_invited_email(ctx: &mut FinalizeContext, event: &Event, user: &User) -> Result<()> {
    let event_url = event_url(ctx, user, event).await?;
    let ics_file = crate::play::to_calendar(Some(event), &ctx.url_prefix.0)?.to_string();
    let email: InvitedEmail<'_> = InvitedEmail {
        event,
        event_url,
        name: &user.name,
        ics_file,
    };
    ctx.email_sender.send(user.mailbox()?, &email).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct InvitedEmail<'a> {
    event: &'a Event,
    name: &'a str,
    event_url: String,
    ics_file: String,
}

impl<'a> EmailMessage for InvitedEmail<'a> {
    fn subject(&self) -> String {
        const FORMAT: &[FormatItem<'_>] =
            format_description!("[day padding:none]. [month repr:long]");
        format!(
            "You're invited to Tau's Game Night on {date}!",
            date = self.event.starts_at.format(FORMAT).unwrap()
        )
    }

    fn template_name(&self) -> String {
        "event/invited".to_string()
    }

    fn attachments(&self) -> Result<Vec<SinglePart>> {
        let ics_attachment = Attachment::new("game-night.ics".to_string())
            .body(self.ics_file.clone(), ContentType::parse("text/calendar")?);
        Ok(vec![ics_attachment])
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

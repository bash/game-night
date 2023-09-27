use super::FinalizeResult::{self, *};
use crate::email::EmailSender;
use crate::poll::Poll;
use crate::users::{User, UserId};
use anyhow::Result;

type Event = crate::event::Event<(), UserId, i64>;

pub(super) async fn send_notification_emails(
    email_sender: &dyn EmailSender,
    result: &FinalizeResult,
) -> Result<()> {
    match result {
        Success(event, invited, not_invited) => {
            for user in invited {
                send_invited_email(email_sender, event, user).await?;
            }
            for user in not_invited {
                send_not_invited_email(email_sender, event, user).await?;
            }
        }
        Failure(poll) => {
            for user in poll.potential_participants() {
                send_failure_email(email_sender, poll, user).await?;
            }
        }
    }
    Ok(())
}

async fn send_invited_email(
    _email_sender: &dyn EmailSender,
    _event: &Event,
    _user: &User,
) -> Result<()> {
    todo!()
}

async fn send_not_invited_email(
    _email_sender: &dyn EmailSender,
    _event: &Event,
    _user: &User,
) -> Result<()> {
    todo!()
}

async fn send_failure_email(
    _email_sender: &dyn EmailSender,
    _poll: &Poll,
    _user: &User,
) -> Result<()> {
    todo!()
}

use super::FinalizeResult;
use crate::email::EmailSender;
use anyhow::Result;

pub(super) async fn send_notification_emails(
    _email_sender: &dyn EmailSender,
    _result: &FinalizeResult,
) -> Result<()> {
    todo!()
}

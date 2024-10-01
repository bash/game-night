use super::FinalizeContext;
use crate::database::{New, Repository};
use crate::event::{Event, StatefulEvent};
use crate::poll::{Answer, AnswerValue, Poll, PollOption};
use crate::users::User;
use anyhow::Result;
use time::Date;

pub(super) async fn veto_date_in_other_polls(
    ctx: &mut FinalizeContext,
    event: &Event,
) -> Result<()> {
    for poll in open_and_pending_polls(&mut *ctx.repository).await? {
        if let Some(option) = find_option(&poll, event.starts_at.date()) {
            ctx.repository
                .add_answers(vec![veto(option, &event.created_by)])
                .await?;
        }
    }
    Ok(())
}

async fn open_and_pending_polls(
    repository: &mut dyn Repository,
) -> Result<impl Iterator<Item = Poll>> {
    Ok(repository
        .get_stateful_events()
        .await?
        .into_iter()
        .filter_map(polling_or_pending))
}

fn polling_or_pending(event: StatefulEvent) -> Option<Poll> {
    if let StatefulEvent::Polling(poll) | StatefulEvent::Pending(poll) = event {
        Some(poll)
    } else {
        None
    }
}

fn find_option(poll: &Poll, date: Date) -> Option<&PollOption> {
    poll.options.iter().find(|o| o.starts_at.date() == date)
}

fn veto(option: &PollOption, user: &User) -> (i64, Answer<New>) {
    let answer = Answer {
        id: (),
        value: AnswerValue::veto(),
        user: user.id,
    };
    (option.id, answer)
}

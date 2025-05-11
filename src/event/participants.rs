use super::{Event, Participant};
use crate::users::{Role, User, UserId};

#[derive(Debug)]
pub(crate) struct VisibleParticipants {
    pub(crate) redacted: bool,
    pub(crate) participants: Vec<Participant>,
}

impl VisibleParticipants {
    pub(crate) fn from_event(event: &Event, user: &User, is_planned: bool) -> Self {
        if may_see_all_participants(event, user, is_planned) {
            Self {
                redacted: false,
                participants: event.participants.clone(),
            }
        } else {
            Self {
                redacted: true,
                participants: find_participant(event, event.created_by.id)
                    .into_iter()
                    .collect(),
            }
        }
    }
}

fn may_see_all_participants(event: &Event, user: &User, is_planned: bool) -> bool {
    event.is_participant(user) || is_planned || user.role == Role::Admin
}

fn find_participant(event: &Event, user_id: UserId) -> Option<Participant> {
    event
        .participants
        .iter()
        .find(|p| p.user.id == user_id)
        .cloned()
}

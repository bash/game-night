use super::EventEmailSender;
use crate::database::Repository;
use crate::email::EmailMessage;
use crate::event::{Event, EventId, EventsQuery, StatefulEvent};
use crate::result::HttpResult;
use crate::template::PageBuilder;
use crate::uri;
use crate::users::User;
use lettre::message::Mailbox;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, FromForm};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

#[get("/event/<id>/leave")]
pub(crate) async fn leave_page(
    id: EventId,
    user: User,
    page: PageBuilder<'_>,
    mut events_query: EventsQuery,
) -> HttpResult<Template> {
    let event = events_query.with_id(id, &user).await?;
    if let Some(StatefulEvent::Planned(event)) = event {
        Ok(page.render("play/leave", context! { event }))
    } else {
        Err(Status::NotFound.into())
    }
}

#[post("/event/<id>/leave", data = "<form>")]
pub(crate) async fn leave_(
    id: EventId,
    user: User,
    form: Form<LeaveFormData<'_>>,
    mut events_query: EventsQuery,
    mut repository: Box<dyn Repository>,
    mut email_sender: Box<dyn EventEmailSender>,
) -> HttpResult<Redirect> {
    let event = events_query.with_id(id, &user).await?;
    let message = form.message.trim();
    if let Some(StatefulEvent::Planned(event)) = event {
        repository.remove_participant(event.id, user.id).await?;
        let email = ParticipantLeftEmail {
            event: &event,
            participant: &user,
            message,
        };
        email_sender.send(&event, &event.created_by, &email).await?;
        Ok(Redirect::to(uri!(crate::event::event_page(id = event.id))))
    } else {
        Err(Status::UnprocessableEntity.into())
    }
}

#[derive(Debug, FromForm)]
pub(crate) struct LeaveFormData<'r> {
    pub(crate) message: &'r str,
}

#[derive(Debug, Serialize)]
struct ParticipantLeftEmail<'a> {
    event: &'a Event,
    participant: &'a User,
    message: &'a str,
}

impl EmailMessage for ParticipantLeftEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "{} cannot make it and has left the event",
            self.participant.name,
        )
    }

    fn template_name(&self) -> String {
        "event/participant-left".to_string()
    }

    fn reply_to(&self) -> Option<Mailbox> {
        self.participant.mailbox().ok()
    }
}

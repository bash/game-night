use super::{
    Event, EventEmailSender, EventId, EventsQuery, LongEventTitleComponent, StatefulEvent,
};
use crate::database::Repository;
use crate::decorations::Random;
use crate::email::{EmailMessage, EmailTemplateContext};
use crate::result::HttpResult;
use crate::template::prelude::*;
use crate::users::{User, UserNameComponent};
use crate::{email_template, uri};
use lettre::message::Mailbox;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, FromForm};

#[get("/event/<id>/leave")]
pub(crate) async fn leave_page(
    id: EventId,
    user: User,
    page: PageContextBuilder<'_>,
    mut events_query: EventsQuery,
) -> HttpResult<Templated<LeavePage>> {
    let event = events_query.with_id(id, &user).await?;
    if let Some(StatefulEvent::Planned(event)) = event {
        let ctx = page.build();
        Ok(Templated(LeavePage { event, ctx }))
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
    email_ctx: EmailTemplateContext,
) -> HttpResult<Redirect> {
    let event = events_query.with_id(id, &user).await?;
    let message = form.message.trim();
    if let Some(StatefulEvent::Planned(event)) = event {
        repository.remove_participant(event.id, user.id).await?;
        let email = ParticipantLeftEmail {
            event: &event,
            participant: &user,
            message,
            random: Random::default(),
            ctx: email_ctx,
        };
        email_sender.send(&event, &event.created_by, &email).await?;
        Ok(Redirect::to(uri!(crate::event::event_page(id = event.id))))
    } else {
        Err(Status::UnprocessableEntity.into())
    }
}

#[derive(Debug, Template)]
#[template(path = "event/leave.html")]
pub(crate) struct LeavePage {
    event: Event,
    ctx: PageContext,
}

#[derive(Debug, FromForm)]
pub(crate) struct LeaveFormData<'r> {
    pub(crate) message: &'r str,
}

email_template! {
    #[template(html_path = "emails/event/participant-left.html", txt_path = "emails/event/participant-left.txt")]
    #[derive(Debug)]
    struct ParticipantLeftEmail<'a> {
        event: &'a Event,
        participant: &'a User,
        message: &'a str,
        random: Random,
        ctx: EmailTemplateContext,
    }
}

impl EmailMessage for ParticipantLeftEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "{} cannot make it and has left the event",
            self.participant.name,
        )
    }

    fn reply_to(&self) -> Option<Mailbox> {
        self.participant.mailbox().ok()
    }
}

use crate::event::{EventViewModel, EventsQuery};
use crate::template::PageBuilder;
use crate::users::User;
use crate::HttpResult;
use itertools::Itertools;
use rocket::get;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;
use std::cmp::Reverse;
use time::OffsetDateTime;

#[get("/archive")]
pub(crate) async fn archive_page(
    user: User,
    page: PageBuilder<'_>,
    mut events: EventsQuery,
) -> HttpResult<Template> {
    let events = events.all(&user).await?;
    let events_by_year: Vec<_> = events
        .into_iter()
        .sorted_by_key(|e| Reverse(e.date()))
        .chunk_by(|e| e.date().year())
        .into_iter()
        .map(|(year, events)| Year {
            year,
            events: events
                .map(|e| EventViewModel::from_event(e, &user))
                .collect(),
        })
        .collect();
    Ok(page.render(
        "play/archive",
        context! {
            events_by_year,
            current_year: OffsetDateTime::now_utc().year()
        },
    ))
}

#[derive(Debug, Serialize)]
struct Year {
    year: i32,
    events: Vec<EventViewModel>,
}

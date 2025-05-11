use crate::event::{EventListComponent, EventViewModel, EventsQuery, StatefulEvent};
use crate::result::HttpResult;
use crate::template_v2::prelude::*;
use crate::users::User;
use itertools::Itertools;
use rocket::get;
use std::cmp::Reverse;
use time::OffsetDateTime;

#[get("/archive")]
pub(crate) async fn archive_page(
    user: User,
    page: PageContextBuilder<'_>,
    mut events: EventsQuery,
) -> HttpResult<Templated<ArchivePage>> {
    let events = events.all(&user).await?;
    let ctx = page.build();
    Ok(Templated(ArchivePage::from_events(events, &user, ctx)))
}

#[derive(Template, Debug)]
#[template(path = "event/archive.html")]
pub(crate) struct ArchivePage {
    events_by_year: Vec<EventsGroup>,
    current_year: i32,
    ctx: PageContext,
}

#[derive(Debug)]
struct EventsGroup {
    year: i32,
    events: Vec<EventViewModel>,
}

impl ArchivePage {
    fn from_events(events: Vec<StatefulEvent>, user: &User, ctx: PageContext) -> Self {
        let events_by_year = group_events_by_year(
            events
                .into_iter()
                .map(|e| EventViewModel::from_event(e, user)),
        );
        let current_year = OffsetDateTime::now_utc().year();
        Self {
            events_by_year,
            current_year,
            ctx,
        }
    }
}

fn group_events_by_year(events: impl Iterator<Item = EventViewModel>) -> Vec<EventsGroup> {
    events
        .sorted_by_key(|e| Reverse(e.date()))
        .chunk_by(|e| e.date().year())
        .into_iter()
        .map(|(year, events)| EventsGroup {
            year,
            events: events.collect(),
        })
        .collect()
}

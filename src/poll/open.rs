use crate::poll::{Poll, PollOption};
use crate::template::{PageBuilder, PageType};
use itertools::Itertools;
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

pub(super) fn open_poll_page(page: PageBuilder<'_>, poll: Poll) -> Template {
    let options_by_month: Vec<_> = poll
        .options
        .iter()
        .cloned()
        .group_by(|o| o.datetime.month())
        .into_iter()
        .map(|(month, options)| PollOptionGroup {
            name: month.to_string(),
            options: options.collect(),
        })
        .collect();
    page.type_(PageType::Poll)
        .render("poll/open", context! { poll, options_by_month })
}

#[derive(Debug, Serialize)]
struct PollOptionGroup {
    name: String,
    options: Vec<PollOption>,
}

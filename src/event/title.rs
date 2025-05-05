use super::EventLike;
use crate::template_v2::prelude::*;

#[derive(Debug, Template)]
#[template(
    ext = "html",
    source = r#"
    {% let title = event.details().title() %}
    {% if !title.is_empty() %}
        <em>Tau's Game Night «{{title}}»</em>
    {% else %}
        {{untitled_prefix}} <em>Tau's Game Night</em>
    {% endif %}
"#
)]
pub(crate) struct LongEventTitleComponent<'a, E: EventLike> {
    event: &'a E,
    untitled_prefix: &'a str,
}

impl<'a, E: EventLike> LongEventTitleComponent<'a, E> {
    pub(crate) fn for_event(event: &'a E) -> Self {
        Self {
            event,
            untitled_prefix: "",
        }
    }

    pub(crate) fn untitled_prefix(mut self, prefix: &'a str) -> Self {
        self.untitled_prefix = prefix;
        self
    }
}

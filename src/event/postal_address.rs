use super::Location;
use crate::template_v2::prelude::*;

#[derive(Template, Debug)]
#[template(path = "event/postal-address.html")]
pub(crate) struct PostalAddressComponent<'a> {
    location: &'a Location,
}

impl<'a> PostalAddressComponent<'a> {
    pub(crate) fn for_location(location: &'a Location) -> Self {
        Self { location }
    }
}

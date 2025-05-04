use super::Location;
use crate::template_v2::prelude::*;

#[derive(Template, Debug)]
#[template(path = "event/postal-address.html")]
pub(crate) struct PostalAddressComponent<'a> {
    pub(crate) location: &'a Location,
}

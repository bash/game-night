use crate::impl_resolve_for_state;
use anyhow::Result;
use rocket::figment::Figment;

#[derive(Debug, Clone)]
pub(super) struct VapidContact(pub(super) String);

impl VapidContact {
    pub(super) fn from_figment(figment: &Figment) -> Result<Self> {
        Ok(VapidContact(
            figment.focus("web_push").extract_inner("vapid_contact")?,
        ))
    }
}

impl_resolve_for_state!(VapidContact: "VAPID contact");

use crate::impl_resolve_for_state;

pub(crate) type HttpClient = reqwest::Client;

impl_resolve_for_state!(HttpClient: "http client", without_from_request);

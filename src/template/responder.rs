use crate::result::HttpError;
use askama::Template;
use rocket::response::content::RawHtml;
use rocket::response::Responder;

#[derive(Debug)]
pub(crate) struct Templated<T>(pub(crate) T);

impl<'r, 'o: 'r, T> Responder<'r, 'o> for Templated<T>
where
    T: Template,
{
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        self.0
            .render()
            .map(RawHtml)
            .map_err(HttpError::from)
            .respond_to(request)
    }
}

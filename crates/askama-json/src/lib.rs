use askama::{NO_VALUES, Result, Values};

pub use askama;
pub use askama_json_macros::JsonTemplate;
pub use serde_json;

pub trait JsonTemplate {
    fn render(&self) -> Result<serde_json::Value> {
        self.render_with_values(NO_VALUES)
    }

    fn render_with_values(&self, values: &dyn Values) -> Result<serde_json::Value>;
}

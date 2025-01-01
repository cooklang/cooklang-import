use crate::model::Recipe;
use scraper::Html;

mod json_ld;
mod plain_text_llm;

pub use self::json_ld::JsonLdExtractor;
pub use self::plain_text_llm::PlainTextLlmExtractor;

pub trait Extractor {
    fn can_parse(&self, document: &Html) -> bool;
    fn parse(&self, document: &Html) -> Result<Recipe, Box<dyn std::error::Error>>;
}

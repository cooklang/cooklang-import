use crate::model::Recipe;
use scraper::Html;

mod json_ld;
mod plain_text_llm;

pub use self::json_ld::JsonLdExtractor;
pub use self::plain_text_llm::PlainTextLlmExtractor;

pub struct ParsingContext {
    pub url: String,
    pub document: Html,
    pub texts: Option<String>,
}

pub trait Extractor {
    fn can_parse(&self, context: &ParsingContext) -> bool;
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>>;
}

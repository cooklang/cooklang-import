use crate::model::Recipe;
use scraper::Html;

mod html_class;
mod json_ld;
mod microdata;
mod plain_text_llm;

pub use self::html_class::HtmlClassExtractor;
pub use self::json_ld::JsonLdExtractor;
pub use self::microdata::MicroDataExtractor;
pub use self::plain_text_llm::PlainTextLlmExtractor;

pub struct ParsingContext {
    pub url: String,
    pub document: Html,
    pub texts: Option<String>,
}

pub trait Extractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>>;
}

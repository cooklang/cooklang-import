use crate::model::Recipe;
use scraper::Html;

mod json_ld;
mod microdata;

pub use json_ld::JsonLdExtractor;
pub use microdata::MicroDataExtractor;

pub struct ParsingContext {
    pub url: String,
    pub document: Html,
    pub texts: Option<String>,
}

pub trait Extractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>>;
}

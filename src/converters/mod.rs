mod open_ai;

use std::error::Error;

pub trait ConvertToCooklang {
    fn convert(&self, ingredients: &[String], steps: &str) -> Result<String, Box<dyn Error>>;
}

pub use open_ai::OpenAIConverter;

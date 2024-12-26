mod open_ai;

use std::error::Error;

#[async_trait::async_trait]
pub trait ConvertToCooklang {
    async fn convert(&self, ingredients: &[String], steps: &str) -> Result<String, Box<dyn Error>>;
}

pub use open_ai::OpenAIConverter;

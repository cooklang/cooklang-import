use crate::url_to_text::text::TextExtractor;
use std::error::Error;

pub async fn process(
    text: &str,
    extract: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    if extract {
        // Run through LLM extractor
        TextExtractor::extract(text, "direct-input").await
    } else {
        // Assume already formatted
        Ok(text.to_string())
    }
}

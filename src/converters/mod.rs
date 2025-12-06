mod anthropic;
mod azure_openai;
mod google;
mod ollama;
mod open_ai;
mod prompt;

pub use anthropic::AnthropicConverter;
pub use azure_openai::AzureOpenAiConverter;
pub use google::GoogleConverter;
pub use ollama::OllamaConverter;
pub use open_ai::OpenAiConverter;
pub use prompt::COOKLANG_CONVERTER_PROMPT;

use async_trait::async_trait;
use std::error::Error;

/// Unified trait for all converters that transform recipe text to Cooklang format
#[async_trait]
pub trait Converter: Send + Sync {
    /// Get the converter name (e.g., "open_ai", "anthropic")
    fn name(&self) -> &str;

    /// Convert recipe ingredients and instructions to Cooklang format
    async fn convert(
        &self,
        ingredients_and_instructions: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>>;
}

/// Factory function to create a converter by name
///
/// # Arguments
/// * `name` - The converter name (e.g., "open_ai", "anthropic")
/// * `config` - Provider configuration
///
/// # Returns
/// * `Some(Box<dyn Converter>)` if the converter exists
/// * `None` if the converter name is not recognized
pub fn create_converter(
    name: &str,
    config: &crate::config::ProviderConfig,
) -> Option<Box<dyn Converter>> {
    match name {
        "open_ai" => OpenAiConverter::new(config)
            .ok()
            .map(|c| Box::new(c) as Box<dyn Converter>),
        "anthropic" => AnthropicConverter::new(config)
            .ok()
            .map(|c| Box::new(c) as Box<dyn Converter>),
        "azure_openai" => AzureOpenAiConverter::new(config)
            .ok()
            .map(|c| Box::new(c) as Box<dyn Converter>),
        "google" => GoogleConverter::new(config)
            .ok()
            .map(|c| Box::new(c) as Box<dyn Converter>),
        "ollama" => OllamaConverter::new(config)
            .ok()
            .map(|c| Box::new(c) as Box<dyn Converter>),
        _ => None,
    }
}

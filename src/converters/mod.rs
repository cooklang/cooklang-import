mod anthropic;
mod azure_openai;
mod evaluator;
mod google;
#[cfg(feature = "local-model")]
pub(crate) mod local;
mod ollama;
mod open_ai;
mod prompt;

pub use anthropic::AnthropicConverter;
pub use azure_openai::AzureOpenAiConverter;
pub use evaluator::{inject_evaluation, COOKLANG_EVALUATOR_PROMPT};
pub use google::GoogleConverter;
#[cfg(feature = "local-model")]
pub use local::LocalConverter;
pub use ollama::OllamaConverter;
pub use open_ai::OpenAiConverter;
pub use prompt::{inject_recipe, COOKLANG_CONVERTER_PROMPT};

use async_trait::async_trait;
use serde::Serialize;
use std::error::Error;

/// Metadata about token usage from LLM conversion
#[derive(Debug, Clone, Default, Serialize)]
pub struct TokenUsage {
    /// Number of tokens in the input/prompt
    pub input_tokens: Option<u32>,
    /// Number of tokens in the output/completion
    pub output_tokens: Option<u32>,
}

/// Metadata about the conversion operation
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConversionMetadata {
    /// The model version/name that was used for conversion
    pub model_version: Option<String>,
    /// Token usage information
    pub tokens_used: TokenUsage,
    /// Time taken for the conversion in milliseconds
    pub latency_ms: u64,
}

/// Result of a conversion operation including the converted text and metadata
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// The converted Cooklang text
    pub content: String,
    /// Metadata about the conversion
    pub metadata: ConversionMetadata,
}

/// Unified trait for all converters that transform recipe text to Cooklang format
#[async_trait]
pub trait Converter: Send + Sync {
    /// Get the converter name (e.g., "open_ai", "anthropic")
    fn name(&self) -> &str;

    /// Convert recipe ingredients and instructions to Cooklang format
    async fn convert(
        &self,
        ingredients_and_instructions: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>>;
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
        #[cfg(feature = "local-model")]
        "local" => Some(Box::new(local::LocalConverter::new()) as Box<dyn Converter>),
        _ => None,
    }
}

#[cfg(all(test, feature = "local-model"))]
mod local_registration_tests {
    use super::*;

    #[test]
    fn create_converter_recognizes_local() {
        let config = crate::config::ProviderConfig {
            enabled: true,
            model: String::new(),
            temperature: 0.0,
            max_tokens: 0,
            api_key: None,
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };
        let converter = create_converter("local", &config).expect("local converter registered");
        assert_eq!(converter.name(), "local");
    }
}

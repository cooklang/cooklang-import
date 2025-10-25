mod anthropic;
mod azure_openai;
mod factory;
mod fallback;
mod google;
mod ollama;
mod open_ai;
mod prompt;

pub use anthropic::AnthropicProvider;
pub use azure_openai::AzureOpenAIProvider;
pub use factory::ProviderFactory;
pub use fallback::FallbackProvider;
pub use google::GoogleProvider;
pub use ollama::OllamaProvider;
pub use open_ai::OpenAIProvider;
pub use prompt::COOKLANG_CONVERTER_PROMPT;

use async_trait::async_trait;
use std::error::Error;

/// Unified trait for all LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider name (e.g., "openai", "anthropic")
    fn provider_name(&self) -> &str;

    /// Convert recipe ingredients and instructions to Cooklang format
    async fn convert(
        &self,
        ingredients: &str,
        instructions: &str,
    ) -> Result<String, Box<dyn Error>>;
}

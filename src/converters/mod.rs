mod prompt;

pub use prompt::COOKLANG_CONVERTER_PROMPT;

use async_trait::async_trait;
use std::error::Error;

/// Unified trait for all converters that transform recipe text to Cooklang format
#[async_trait]
pub trait Converter: Send + Sync {
    /// Get the converter name (e.g., "open_ai", "anthropic")
    fn name(&self) -> &str;

    /// Convert recipe ingredients and instructions to Cooklang format
    async fn convert(&self, ingredients_and_instructions: &str) -> Result<String, Box<dyn Error + Send + Sync>>;
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
    _name: &str,
    _config: &crate::config::ProviderConfig,
) -> Option<Box<dyn Converter>> {
    // TODO: Implement converter creation in subsequent tasks
    // This is a placeholder for now
    None
}

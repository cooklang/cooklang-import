//! UniFFI bindings for cooklang-import
//!
//! This module provides FFI-compatible types and functions for use with iOS and Android.
//! It wraps the async Rust API with synchronous functions that manage their own tokio runtime.

use std::fmt;
use std::time::Duration;

use crate::{ImportError, RecipeComponents};

// Re-export UniFFI macro
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

/// FFI-compatible recipe components structure
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FfiRecipeComponents {
    /// Recipe text (ingredients + instructions)
    pub text: String,
    /// YAML-formatted metadata (without --- delimiters)
    pub metadata: String,
    /// Recipe name/title
    pub name: String,
}

impl From<RecipeComponents> for FfiRecipeComponents {
    fn from(components: RecipeComponents) -> Self {
        FfiRecipeComponents {
            text: components.text,
            metadata: components.metadata,
            name: components.name,
        }
    }
}

impl From<FfiRecipeComponents> for RecipeComponents {
    fn from(ffi: FfiRecipeComponents) -> Self {
        RecipeComponents {
            text: ffi.text,
            metadata: ffi.metadata,
            name: ffi.name,
        }
    }
}

/// FFI-compatible LLM provider enum
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FfiLlmProvider {
    OpenAI,
    Anthropic,
    Google,
    AzureOpenAI,
    Ollama,
}

impl From<FfiLlmProvider> for crate::LlmProvider {
    fn from(provider: FfiLlmProvider) -> Self {
        match provider {
            FfiLlmProvider::OpenAI => crate::LlmProvider::OpenAI,
            FfiLlmProvider::Anthropic => crate::LlmProvider::Anthropic,
            FfiLlmProvider::Google => crate::LlmProvider::Google,
            FfiLlmProvider::AzureOpenAI => crate::LlmProvider::AzureOpenAI,
            FfiLlmProvider::Ollama => crate::LlmProvider::Ollama,
        }
    }
}

/// FFI-compatible import result
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FfiImportResult {
    /// Recipe converted to Cooklang format
    Cooklang { content: String },
    /// Recipe components extracted but not converted
    Components { components: FfiRecipeComponents },
}

/// FFI-compatible error type
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum FfiImportError {
    /// Failed to fetch recipe from URL
    FetchError { reason: String },
    /// Failed to parse recipe from webpage
    ParseError { reason: String },
    /// No extractor could successfully parse the recipe
    NoExtractorMatched { reason: String },
    /// Failed to convert recipe to Cooklang format
    ConversionError { reason: String },
    /// Invalid input provided
    InvalidInput { reason: String },
    /// Builder configuration error
    BuilderError { reason: String },
    /// Configuration error
    ConfigError { reason: String },
    /// Runtime error (tokio)
    RuntimeError { reason: String },
}

impl fmt::Display for FfiImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiImportError::FetchError { reason } => write!(f, "Fetch error: {}", reason),
            FfiImportError::ParseError { reason } => write!(f, "Parse error: {}", reason),
            FfiImportError::NoExtractorMatched { reason } => {
                write!(f, "No extractor matched: {}", reason)
            }
            FfiImportError::ConversionError { reason } => {
                write!(f, "Conversion error: {}", reason)
            }
            FfiImportError::InvalidInput { reason } => write!(f, "Invalid input: {}", reason),
            FfiImportError::BuilderError { reason } => write!(f, "Builder error: {}", reason),
            FfiImportError::ConfigError { reason } => write!(f, "Config error: {}", reason),
            FfiImportError::RuntimeError { reason } => write!(f, "Runtime error: {}", reason),
        }
    }
}

impl std::error::Error for FfiImportError {}

impl From<ImportError> for FfiImportError {
    fn from(err: ImportError) -> Self {
        match err {
            ImportError::FetchError(e) => FfiImportError::FetchError {
                reason: e.to_string(),
            },
            ImportError::ParseError(msg) => FfiImportError::ParseError { reason: msg },
            ImportError::NoExtractorMatched => FfiImportError::NoExtractorMatched {
                reason: "No extractor could parse the recipe from this webpage".to_string(),
            },
            ImportError::ConversionError(msg) => FfiImportError::ConversionError { reason: msg },
            ImportError::InvalidMarkdown(msg) => FfiImportError::InvalidInput { reason: msg },
            ImportError::BuilderError(msg) => FfiImportError::BuilderError { reason: msg },
            ImportError::ExtractionError(msg) => FfiImportError::ParseError { reason: msg },
            ImportError::HeaderError(e) => FfiImportError::FetchError {
                reason: e.to_string(),
            },
            ImportError::EnvError(e) => FfiImportError::ConfigError {
                reason: e.to_string(),
            },
            ImportError::ConfigError(e) => FfiImportError::ConfigError {
                reason: e.to_string(),
            },
        }
    }
}

/// Configuration for importing recipes
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FfiImportConfig {
    /// Optional LLM provider (uses default if not specified)
    pub provider: Option<FfiLlmProvider>,
    /// Optional API key (uses environment variable if not specified)
    pub api_key: Option<String>,
    /// Optional model name (uses provider default if not specified)
    pub model: Option<String>,
    /// Optional timeout in seconds (uses default if not specified)
    pub timeout_seconds: Option<u64>,
    /// If true, only extract recipe without converting to Cooklang
    pub extract_only: bool,
}

/// Create a new tokio runtime for FFI calls
fn create_runtime() -> Result<tokio::runtime::Runtime, FfiImportError> {
    tokio::runtime::Runtime::new().map_err(|e| FfiImportError::RuntimeError {
        reason: format!("Failed to create async runtime: {}", e),
    })
}

/// Import a recipe from a URL
///
/// # Arguments
/// * `url` - The URL of the recipe webpage
/// * `config` - Optional configuration for the import
///
/// # Returns
/// An `FfiImportResult` containing either Cooklang text or a Recipe struct
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn import_from_url(
    url: String,
    config: Option<FfiImportConfig>,
) -> Result<FfiImportResult, FfiImportError> {
    let rt = create_runtime()?;
    rt.block_on(async { import_from_url_async(&url, config).await })
}

async fn import_from_url_async(
    url: &str,
    config: Option<FfiImportConfig>,
) -> Result<FfiImportResult, FfiImportError> {
    let config = config.unwrap_or_default();

    let mut builder = crate::RecipeImporter::builder().url(url);

    if let Some(provider) = config.provider {
        builder = builder.provider(provider.into());
    }

    if let Some(api_key) = config.api_key {
        builder = builder.api_key(api_key);
    }

    if let Some(model) = config.model {
        builder = builder.model(model);
    }

    if let Some(timeout_secs) = config.timeout_seconds {
        builder = builder.timeout(Duration::from_secs(timeout_secs));
    }

    if config.extract_only {
        builder = builder.extract_only();
    }

    let result = builder.build().await?;

    Ok(match result {
        crate::ImportResult::Cooklang { content, .. } => FfiImportResult::Cooklang { content },
        crate::ImportResult::Components(components) => FfiImportResult::Components {
            components: components.into(),
        },
    })
}

/// Convert plain text to Cooklang format
///
/// # Arguments
/// * `text` - The recipe text in plain format
/// * `config` - Optional configuration for the conversion
///
/// # Returns
/// A string containing the recipe in Cooklang format
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn convert_text_to_cooklang(
    text: String,
    config: Option<FfiImportConfig>,
) -> Result<String, FfiImportError> {
    let rt = create_runtime()?;
    rt.block_on(async { convert_text_async(&text, config).await })
}

async fn convert_text_async(
    text: &str,
    config: Option<FfiImportConfig>,
) -> Result<String, FfiImportError> {
    let config = config.unwrap_or_default();

    let mut builder = crate::RecipeImporter::builder().text(text);

    if let Some(provider) = config.provider {
        builder = builder.provider(provider.into());
    }

    if let Some(api_key) = config.api_key {
        builder = builder.api_key(api_key);
    }

    if let Some(model) = config.model {
        builder = builder.model(model);
    }

    let result = builder.build().await?;

    match result {
        crate::ImportResult::Cooklang { content, .. } => Ok(content),
        crate::ImportResult::Components(_) => Err(FfiImportError::BuilderError {
            reason: "Unexpected components result when converting text".to_string(),
        }),
    }
}

/// Convert an image to Cooklang format using OCR
///
/// # Arguments
/// * `image_path` - Path to the image file
/// * `config` - Optional configuration for the conversion
///
/// # Returns
/// A string containing the recipe in Cooklang format
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn convert_image_to_cooklang(
    image_path: String,
    config: Option<FfiImportConfig>,
) -> Result<String, FfiImportError> {
    let rt = create_runtime()?;
    rt.block_on(async { convert_image_async(&image_path, config).await })
}

async fn convert_image_async(
    image_path: &str,
    config: Option<FfiImportConfig>,
) -> Result<String, FfiImportError> {
    let config = config.unwrap_or_default();

    let mut builder = crate::RecipeImporter::builder().image_path(image_path);

    if let Some(provider) = config.provider {
        builder = builder.provider(provider.into());
    }

    if let Some(api_key) = config.api_key {
        builder = builder.api_key(api_key);
    }

    if let Some(model) = config.model {
        builder = builder.model(model);
    }

    let result = builder.build().await?;

    match result {
        crate::ImportResult::Cooklang { content, .. } => Ok(content),
        crate::ImportResult::Components(_) => Err(FfiImportError::BuilderError {
            reason: "Unexpected components result when converting image".to_string(),
        }),
    }
}

/// Extract recipe components from a URL without converting to Cooklang format
///
/// # Arguments
/// * `url` - The URL of the recipe webpage
/// * `timeout_seconds` - Optional timeout in seconds
///
/// # Returns
/// An `FfiRecipeComponents` struct containing the extracted recipe data
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn extract_recipe_from_url(
    url: String,
    timeout_seconds: Option<u64>,
) -> Result<FfiRecipeComponents, FfiImportError> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let mut builder = crate::RecipeImporter::builder().url(&url).extract_only();

        if let Some(timeout_secs) = timeout_seconds {
            builder = builder.timeout(Duration::from_secs(timeout_secs));
        }

        let result = builder.build().await?;

        match result {
            crate::ImportResult::Components(components) => Ok(components.into()),
            crate::ImportResult::Cooklang { .. } => Err(FfiImportError::BuilderError {
                reason: "Unexpected Cooklang result when extracting".to_string(),
            }),
        }
    })
}

/// Simple import from URL with default settings
///
/// This is a convenience function that imports a recipe from a URL
/// using default settings and returns the Cooklang-formatted result.
///
/// # Arguments
/// * `url` - The URL of the recipe webpage
///
/// # Returns
/// A string containing the recipe in Cooklang format
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn simple_import(url: String) -> Result<String, FfiImportError> {
    import_from_url(url, None).map(|result| match result {
        FfiImportResult::Cooklang { content } => content,
        FfiImportResult::Components { components } => components.text,
    })
}

/// Get the library version
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Check if a provider is available (has required environment variables)
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn is_provider_available(provider: FfiLlmProvider) -> bool {
    match provider {
        FfiLlmProvider::OpenAI => std::env::var("OPENAI_API_KEY").is_ok(),
        FfiLlmProvider::Anthropic => std::env::var("ANTHROPIC_API_KEY").is_ok(),
        FfiLlmProvider::Google => std::env::var("GOOGLE_API_KEY").is_ok(),
        FfiLlmProvider::AzureOpenAI => {
            std::env::var("AZURE_OPENAI_API_KEY").is_ok()
                && std::env::var("AZURE_OPENAI_ENDPOINT").is_ok()
        }
        FfiLlmProvider::Ollama => {
            // Ollama doesn't require API key, check if base URL is set or use default
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_recipe_components_conversion() {
        let components = RecipeComponents {
            text: "2 eggs\n1 cup flour\n\nMix together and bake.".to_string(),
            metadata: "author: Chef".to_string(),
            name: "Test Recipe".to_string(),
        };

        let ffi_components: FfiRecipeComponents = components.clone().into();
        assert_eq!(ffi_components.name, "Test Recipe");
        assert_eq!(ffi_components.metadata, "author: Chef");
        assert!(ffi_components.text.contains("2 eggs"));

        let back: RecipeComponents = ffi_components.into();
        assert_eq!(back.name, components.name);
        assert_eq!(back.metadata, components.metadata);
        assert_eq!(back.text, components.text);
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }
}

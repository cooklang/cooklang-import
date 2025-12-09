//! UniFFI bindings for cooklang-import
//!
//! This module provides FFI-compatible types and functions for use with iOS and Android.
//! It wraps the async Rust API with synchronous functions that manage their own tokio runtime.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::{ImportError, Recipe};

// Re-export UniFFI macro
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

/// FFI-compatible recipe structure
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FfiRecipe {
    /// Recipe name/title
    pub name: String,
    /// Recipe description (empty string if none)
    pub description: String,
    /// List of image URLs
    pub images: Vec<String>,
    /// List of ingredients
    pub ingredients: Vec<String>,
    /// Recipe instructions
    pub instructions: String,
    /// Metadata as key-value pairs
    pub metadata: Vec<FfiKeyValue>,
}

/// Key-value pair for metadata (since HashMap isn't directly supported in UniFFI)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct FfiKeyValue {
    pub key: String,
    pub value: String,
}

impl From<Recipe> for FfiRecipe {
    fn from(recipe: Recipe) -> Self {
        FfiRecipe {
            name: recipe.name,
            description: recipe.description.unwrap_or_default(),
            images: recipe.image,
            ingredients: recipe.ingredients,
            instructions: recipe.instructions,
            metadata: recipe
                .metadata
                .into_iter()
                .map(|(key, value)| FfiKeyValue { key, value })
                .collect(),
        }
    }
}

impl From<FfiRecipe> for Recipe {
    fn from(ffi: FfiRecipe) -> Self {
        Recipe {
            name: ffi.name,
            description: if ffi.description.is_empty() {
                None
            } else {
                Some(ffi.description)
            },
            image: ffi.images,
            ingredients: ffi.ingredients,
            instructions: ffi.instructions,
            metadata: ffi
                .metadata
                .into_iter()
                .map(|kv| (kv.key, kv.value))
                .collect(),
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
    /// Recipe extracted but not converted
    Recipe { recipe: FfiRecipe },
}

/// FFI-compatible error type
#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error))]
pub enum FfiImportError {
    /// Failed to fetch recipe from URL
    FetchError { message: String },
    /// Failed to parse recipe from webpage
    ParseError { message: String },
    /// No extractor could successfully parse the recipe
    NoExtractorMatched { message: String },
    /// Failed to convert recipe to Cooklang format
    ConversionError { message: String },
    /// Invalid input provided
    InvalidInput { message: String },
    /// Builder configuration error
    BuilderError { message: String },
    /// Configuration error
    ConfigError { message: String },
    /// Runtime error (tokio)
    RuntimeError { message: String },
}

impl fmt::Display for FfiImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FfiImportError::FetchError { message } => write!(f, "Fetch error: {}", message),
            FfiImportError::ParseError { message } => write!(f, "Parse error: {}", message),
            FfiImportError::NoExtractorMatched { message } => {
                write!(f, "No extractor matched: {}", message)
            }
            FfiImportError::ConversionError { message } => {
                write!(f, "Conversion error: {}", message)
            }
            FfiImportError::InvalidInput { message } => write!(f, "Invalid input: {}", message),
            FfiImportError::BuilderError { message } => write!(f, "Builder error: {}", message),
            FfiImportError::ConfigError { message } => write!(f, "Config error: {}", message),
            FfiImportError::RuntimeError { message } => write!(f, "Runtime error: {}", message),
        }
    }
}

impl std::error::Error for FfiImportError {}

impl From<ImportError> for FfiImportError {
    fn from(err: ImportError) -> Self {
        match err {
            ImportError::FetchError(e) => FfiImportError::FetchError {
                message: e.to_string(),
            },
            ImportError::ParseError(msg) => FfiImportError::ParseError { message: msg },
            ImportError::NoExtractorMatched => FfiImportError::NoExtractorMatched {
                message: "No extractor could parse the recipe from this webpage".to_string(),
            },
            ImportError::ConversionError(msg) => FfiImportError::ConversionError { message: msg },
            ImportError::InvalidMarkdown(msg) => FfiImportError::InvalidInput { message: msg },
            ImportError::BuilderError(msg) => FfiImportError::BuilderError { message: msg },
            ImportError::HeaderError(e) => FfiImportError::FetchError {
                message: e.to_string(),
            },
            ImportError::EnvError(e) => FfiImportError::ConfigError {
                message: e.to_string(),
            },
            ImportError::ConfigError(e) => FfiImportError::ConfigError {
                message: e.to_string(),
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
        message: format!("Failed to create async runtime: {}", e),
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
        crate::ImportResult::Recipe(recipe) => FfiImportResult::Recipe {
            recipe: recipe.into(),
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
        crate::ImportResult::Recipe(_) => Err(FfiImportError::BuilderError {
            message: "Unexpected recipe result when converting text".to_string(),
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
        crate::ImportResult::Recipe(_) => Err(FfiImportError::BuilderError {
            message: "Unexpected recipe result when converting image".to_string(),
        }),
    }
}

/// Extract a recipe from a URL without converting to Cooklang format
///
/// # Arguments
/// * `url` - The URL of the recipe webpage
/// * `timeout_seconds` - Optional timeout in seconds
///
/// # Returns
/// An `FfiRecipe` struct containing the extracted recipe data
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn extract_recipe_from_url(
    url: String,
    timeout_seconds: Option<u64>,
) -> Result<FfiRecipe, FfiImportError> {
    let rt = create_runtime()?;
    rt.block_on(async {
        let timeout = timeout_seconds.map(Duration::from_secs);
        let recipe = crate::fetch_recipe_with_timeout(&url, timeout).await?;
        Ok(recipe.into())
    })
}

/// Generate Cooklang frontmatter from metadata
///
/// # Arguments
/// * `metadata` - List of key-value pairs
///
/// # Returns
/// A string containing the frontmatter in Cooklang format
#[cfg_attr(feature = "uniffi", uniffi::export)]
pub fn generate_frontmatter(metadata: Vec<FfiKeyValue>) -> String {
    let map: HashMap<String, String> = metadata.into_iter().map(|kv| (kv.key, kv.value)).collect();
    crate::generate_frontmatter(&map)
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
        FfiImportResult::Recipe { recipe } => recipe.instructions,
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
    fn test_ffi_recipe_conversion() {
        let recipe = Recipe {
            name: "Test Recipe".to_string(),
            description: Some("A test".to_string()),
            image: vec!["http://example.com/image.jpg".to_string()],
            ingredients: vec!["2 eggs".to_string(), "1 cup flour".to_string()],
            instructions: "Mix together and bake.".to_string(),
            metadata: [("author".to_string(), "Chef".to_string())]
                .into_iter()
                .collect(),
        };

        let ffi_recipe: FfiRecipe = recipe.clone().into();
        assert_eq!(ffi_recipe.name, "Test Recipe");
        assert_eq!(ffi_recipe.description, "A test");
        assert_eq!(ffi_recipe.images.len(), 1);
        assert_eq!(ffi_recipe.ingredients.len(), 2);
        assert_eq!(ffi_recipe.metadata.len(), 1);

        let back: Recipe = ffi_recipe.into();
        assert_eq!(back.name, recipe.name);
        assert_eq!(back.description, recipe.description);
        assert_eq!(back.ingredients, recipe.ingredients);
        assert_eq!(back.instructions, recipe.instructions);
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_generate_frontmatter_ffi() {
        let metadata = vec![
            FfiKeyValue {
                key: "author".to_string(),
                value: "Chef".to_string(),
            },
            FfiKeyValue {
                key: "servings".to_string(),
                value: "4".to_string(),
            },
        ];

        let frontmatter = generate_frontmatter(metadata);
        assert!(frontmatter.contains("author: Chef"));
        assert!(frontmatter.contains("servings: 4"));
    }
}

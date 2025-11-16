pub mod builder;
pub mod config;
pub mod error;
pub mod extractors;
pub mod model;
pub mod ocr;
pub mod providers;

// Public API exports
// Builder API (primary interface)
pub use builder::{ImportResult, LlmProvider, RecipeImporter};
// Error types
pub use error::ImportError;
// Types
pub use model::Recipe;
// Convenience functions (secondary interface) - exported at module level below
// Low-level API (for advanced users) - exported at module level below

use log::debug;
use reqwest::header::{HeaderMap, USER_AGENT};
use scraper::Html;

use crate::extractors::{Extractor, ParsingContext};

/// Fetches and extracts a recipe from a URL (convenience function).
///
/// This is a convenience wrapper around `fetch_recipe_with_timeout` with no timeout.
///
/// # Arguments
/// * `url` - The URL of the recipe webpage to fetch
///
/// # Returns
/// A `Recipe` struct containing the extracted recipe data
///
/// # Errors
/// Returns `ImportError::FetchError` if the URL cannot be fetched
/// Returns `ImportError::NoExtractorMatched` if no extractor can parse the recipe
pub async fn fetch_recipe(url: &str) -> Result<model::Recipe, ImportError> {
    fetch_recipe_with_timeout(url, None).await
}

/// Fetches and extracts a recipe from a URL.
///
/// This function performs the following steps:
/// 1. Fetches the webpage content with appropriate headers
/// 2. Attempts to extract recipe data using multiple extractors in sequence
/// 3. Returns the first successful extraction
///
/// # Arguments
/// * `url` - The URL of the recipe webpage to fetch
/// * `timeout` - Optional timeout for the HTTP request
///
/// # Returns
/// A `Recipe` struct containing the extracted recipe data
///
/// # Errors
/// Returns `ImportError::FetchError` if the URL cannot be fetched
/// Returns `ImportError::NoExtractorMatched` if no extractor can parse the recipe
pub async fn fetch_recipe_with_timeout(
    url: &str,
    timeout: Option<std::time::Duration>,
) -> Result<model::Recipe, ImportError> {
    // Set up headers with a user agent
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse()?);

    // Create client with optional timeout
    let mut client_builder = reqwest::Client::builder();
    if let Some(timeout_duration) = timeout {
        client_builder = client_builder.timeout(timeout_duration);
    }
    let client = client_builder.build()?;

    // Fetch the webpage content with headers
    let body = client
        .get(url)
        .headers(headers.clone())
        .send()
        .await?
        .text()
        .await?;

    let context = ParsingContext {
        url: url.to_string(),
        document: Html::parse_document(&body),
        texts: None,
    };

    let extractors_list: Vec<Box<dyn Extractor>> = vec![
        Box::new(extractors::JsonLdExtractor),
        Box::new(extractors::HtmlClassExtractor),
        Box::new(extractors::PlainTextLlmExtractor),
    ];

    for extractor in extractors_list {
        match extractor.parse(&context) {
            Ok(recipe) => {
                debug!("{:#?}", recipe);
                return Ok(recipe);
            }
            Err(e) => {
                debug!("Extractor failed: {}", e);
            }
        }
    }

    Err(ImportError::NoExtractorMatched)
}

pub fn generate_frontmatter(metadata: &std::collections::HashMap<String, String>) -> String {
    if metadata.is_empty() {
        return String::new();
    }

    let mut frontmatter = String::from("---\n");

    // Sort keys for consistent output
    let mut keys: Vec<_> = metadata.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = metadata.get(key) {
            // Escape values that contain special characters
            if value.contains('\n') || value.contains('"') || value.contains(':') {
                frontmatter.push_str(&format!("{}: \"{}\"\n", key, value.replace('"', "\\\"")));
            } else {
                frontmatter.push_str(&format!("{key}: {value}\n"));
            }
        }
    }

    frontmatter.push_str("---\n\n");
    frontmatter
}

/// Converts a recipe to Cooklang format with explicit configuration.
///
/// This function allows passing API key and model directly instead of relying on
/// config files or environment variables.
///
/// # Arguments
/// * `recipe` - The recipe to convert
/// * `provider_name` - Optional provider name ("openai", "anthropic", "google", "ollama", "azure_openai")
/// * `api_key` - Optional API key to use
/// * `model` - Optional model name to use
///
/// # Returns
/// A string containing the recipe in Cooklang format, including frontmatter
///
/// # Errors
/// Returns `ImportError::ConversionError` if the conversion fails
pub async fn convert_recipe_with_config(
    recipe: &model::Recipe,
    provider_name: Option<&str>,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<String, ImportError> {
    use crate::config::ProviderConfig;
    use crate::providers::{AnthropicProvider, OpenAIProvider};

    let name = provider_name.unwrap_or("anthropic");

    // Create provider based on name
    let converter: Box<dyn crate::providers::LlmProvider> = match name {
        "openai" => {
            if let Some(key) = api_key {
                // Create config with provided key
                let config = ProviderConfig {
                    enabled: true,
                    model: model.unwrap_or_else(|| "gpt-4".to_string()),
                    temperature: 0.7,
                    max_tokens: 4000,
                    api_key: Some(key),
                    base_url: None,
                    endpoint: None,
                    deployment_name: None,
                    api_version: None,
                    project_id: None,
                };
                Box::new(OpenAIProvider::new(&config).map_err(|e| {
                    ImportError::ConversionError(format!("Failed to create OpenAI provider: {}", e))
                })?)
            } else {
                Box::new(OpenAIProvider::from_env().map_err(|e| {
                    ImportError::ConversionError(format!("Failed to create OpenAI provider: {}", e))
                })?)
            }
        }
        "anthropic" => {
            // Create config with provided key or None (will fall back to env)
            let config = ProviderConfig {
                enabled: true,
                model: model.unwrap_or_else(|| "claude-sonnet-4-20250514".to_string()),
                temperature: 0.7,
                max_tokens: 4000,
                api_key,
                base_url: None,
                endpoint: None,
                deployment_name: None,
                api_version: None,
                project_id: None,
            };
            Box::new(AnthropicProvider::new(&config)
                .map_err(|e| ImportError::ConversionError(format!("Failed to create Anthropic provider: {}. Make sure ANTHROPIC_API_KEY is set or pass api_key to builder.", e)))?)
        }
        _ => {
            return Err(ImportError::ConversionError(format!(
                "Provider '{}' requires a config.toml file or use convert_recipe_with_provider",
                name
            )));
        }
    };

    // Convert using the provider
    let mut cooklang_recipe = converter
        .convert(&recipe.content)
        .await
        .map_err(|e| ImportError::ConversionError(e.to_string()))?;

    // Prepend frontmatter if there's metadata
    let frontmatter = generate_frontmatter(&recipe.metadata);
    if !frontmatter.is_empty() {
        cooklang_recipe = format!("{}{}", frontmatter, cooklang_recipe);
    }

    Ok(cooklang_recipe)
}

/// Converts a recipe to Cooklang format using a custom provider.
///
/// This function converts recipe ingredients and instructions from markdown
/// format to Cooklang format using the specified LLM provider.
///
/// # Arguments
/// * `recipe` - The recipe to convert
/// * `provider_name` - Optional provider name ("openai", "anthropic", "google", "ollama", "azure_openai")
///   If None, uses the default provider from config
///
/// # Returns
/// A string containing the recipe in Cooklang format, including frontmatter
///
/// # Errors
/// Returns `ImportError::ConversionError` if the conversion fails
/// Returns `ImportError::ConfigError` if the configuration is invalid
pub async fn convert_recipe_with_provider(
    recipe: &model::Recipe,
    provider_name: Option<&str>,
) -> Result<String, ImportError> {
    use crate::config::AiConfig;
    use crate::providers::{OpenAIProvider, ProviderFactory};

    // Try to load config, but continue if it fails (will use env vars)
    let config_result = AiConfig::load();

    let converter: Box<dyn crate::providers::LlmProvider> = match config_result {
        Ok(config) => {
            // Use provided name or fall back to default provider from config
            let name = provider_name.unwrap_or(&config.default_provider);

            // Get provider config
            let provider_config = config.providers.get(name).ok_or_else(|| {
                ImportError::ConversionError(format!(
                    "Provider '{}' not found in configuration",
                    name
                ))
            })?;

            // Create provider from factory
            ProviderFactory::create(name, provider_config)
                .map_err(|e| ImportError::ConversionError(e.to_string()))?
        }
        Err(_) => {
            // No config file - fall back to environment variables
            let name = provider_name.unwrap_or("openai");

            match name {
                "openai" => Box::new(OpenAIProvider::from_env().map_err(|e| {
                    ImportError::ConversionError(format!(
                        "Failed to create OpenAI provider from environment: {}",
                        e
                    ))
                })?),
                "anthropic" => {
                    use crate::providers::AnthropicProvider;
                    // Create a minimal config that will use environment variables
                    let config = crate::config::ProviderConfig {
                        enabled: true,
                        model: "claude-sonnet-4-20250514".to_string(),
                        temperature: 0.7,
                        max_tokens: 4000,
                        api_key: None, // Will fall back to ANTHROPIC_API_KEY env var
                        base_url: None,
                        endpoint: None,
                        deployment_name: None,
                        api_version: None,
                        project_id: None,
                    };
                    Box::new(AnthropicProvider::new(&config)
                        .map_err(|e| ImportError::ConversionError(format!("Failed to create Anthropic provider from environment: {}. Make sure ANTHROPIC_API_KEY is set.", e)))?)
                }
                _ => {
                    return Err(ImportError::ConversionError(
                        format!(
                            "No configuration file found. Provider '{}' requires a config.toml file. \
                            For OpenAI and Anthropic, you can use environment variables (OPENAI_API_KEY or ANTHROPIC_API_KEY).",
                            name
                        )
                    ));
                }
            }
        }
    };

    // Convert using the provider
    let mut cooklang_recipe = converter
        .convert(&recipe.content)
        .await
        .map_err(|e| ImportError::ConversionError(e.to_string()))?;

    // Prepend frontmatter if there's metadata
    let frontmatter = generate_frontmatter(&recipe.metadata);
    if !frontmatter.is_empty() {
        cooklang_recipe = frontmatter + &cooklang_recipe;
    }

    Ok(cooklang_recipe)
}

/// Fetches a recipe from a URL and converts it to Cooklang format.
///
/// This is a convenience function that wraps the builder API for simple use cases.
/// For more control (e.g., custom provider, timeout), use `RecipeImporter::builder()` directly.
///
/// # Arguments
/// * `url` - The URL of the recipe to fetch
///
/// # Returns
/// A `String` containing the recipe in Cooklang format
///
/// # Errors
/// Returns `ImportError` if the URL cannot be fetched, parsed, or converted
///
/// # Examples
/// ```no_run
/// use cooklang_import::import_from_url;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let cooklang = import_from_url("https://example.com/recipe").await?;
///     println!("{}", cooklang);
///     Ok(())
/// }
/// ```
pub async fn import_from_url(url: &str) -> Result<String, ImportError> {
    match RecipeImporter::builder().url(url).build().await? {
        builder::ImportResult::Cooklang(s) => Ok(s),
        builder::ImportResult::Recipe(_) => unreachable!("Default mode is Cooklang"),
    }
}

/// Fetches a recipe from a URL and returns it as a Recipe struct.
///
/// This extracts the recipe without converting to Cooklang format.
/// For more control, use `RecipeImporter::builder()` directly.
///
/// # Arguments
/// * `url` - The URL of the recipe to fetch
///
/// # Returns
/// A `Recipe` struct containing the recipe content
///
/// # Errors
/// Returns `ImportError` if the URL cannot be fetched or parsed
///
/// # Examples
/// ```no_run
/// use cooklang_import::extract_recipe_from_url;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let recipe = extract_recipe_from_url("https://example.com/recipe").await?;
///     println!("Content: {}", recipe.content);
///     Ok(())
/// }
/// ```
pub async fn extract_recipe_from_url(url: &str) -> Result<Recipe, ImportError> {
    match RecipeImporter::builder()
        .url(url)
        .extract_only()
        .build()
        .await?
    {
        builder::ImportResult::Recipe(r) => Ok(r),
        builder::ImportResult::Cooklang(_) => unreachable!("extract_only sets Recipe mode"),
    }
}

/// Converts plain text to Cooklang format.
///
/// This is a convenience function that wraps the builder API.
/// Use this when you have a recipe in unstructured plain text format.
/// The LLM will parse the text to extract ingredients and instructions.
/// For more control, use `RecipeImporter::builder()` directly.
///
/// # Arguments
/// * `text` - The recipe text in plain format
///
/// # Returns
/// A `String` containing the recipe in Cooklang format
///
/// # Errors
/// Returns `ImportError` if the text is invalid or conversion fails
///
/// # Examples
/// ```no_run
/// use cooklang_import::convert_text_to_cooklang;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let recipe_text = "Take 2 eggs and 1 cup of flour. Mix them together and bake at 350F for 30 minutes.";
///     let cooklang = convert_text_to_cooklang(recipe_text).await?;
///     println!("{}", cooklang);
///     Ok(())
/// }
/// ```
pub async fn convert_text_to_cooklang(text: &str) -> Result<String, ImportError> {
    match RecipeImporter::builder().text(text).build().await? {
        builder::ImportResult::Cooklang(s) => Ok(s),
        builder::ImportResult::Recipe(_) => unreachable!("Text + Cooklang mode"),
    }
}

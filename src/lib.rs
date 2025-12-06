pub mod builder;
pub mod config;
pub mod converters;
pub mod error;
pub mod images_to_text;
pub mod model;
pub mod pipelines;
pub mod url_to_text;

#[cfg(feature = "uniffi")]
pub mod uniffi_bindings;

// Re-export UniFFI types when feature is enabled
#[cfg(feature = "uniffi")]
pub use uniffi_bindings::*;

// Re-exports for convenience
pub use builder::{ImportResult, LlmProvider, RecipeImporter, RecipeImporterBuilder};
pub use config::AiConfig;
pub use error::ImportError;
pub use images_to_text::ImageSource;
pub use model::Recipe;

// Convenience functions using the new architecture

/// Helper function to generate YAML frontmatter from metadata
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

/// Fetches and extracts a recipe from a URL (convenience function).
///
/// This is a simplified wrapper that uses the new pipeline architecture.
/// For more control, use `RecipeImporter::builder()` directly.
///
/// # Arguments
/// * `url` - The URL of the recipe webpage to fetch
///
/// # Returns
/// A `Recipe` struct containing the extracted recipe data
///
/// # Errors
/// Returns `ImportError` if the URL cannot be fetched or parsed
pub async fn fetch_recipe(url: &str) -> Result<model::Recipe, ImportError> {
    extract_recipe_from_url(url).await
}

/// Fetches and extracts a recipe from a URL with timeout.
///
/// This is a wrapper that uses the new pipeline architecture.
/// For more control, use `RecipeImporter::builder()` directly.
///
/// # Arguments
/// * `url` - The URL of the recipe webpage to fetch
/// * `timeout` - Optional timeout for the HTTP request
///
/// # Returns
/// A `Recipe` struct containing the extracted recipe data
///
/// # Errors
/// Returns `ImportError` if the URL cannot be fetched or parsed
pub async fn fetch_recipe_with_timeout(
    url: &str,
    timeout: Option<std::time::Duration>,
) -> Result<model::Recipe, ImportError> {
    let mut builder = RecipeImporter::builder().url(url).extract_only();

    if let Some(t) = timeout {
        builder = builder.timeout(t);
    }

    match builder.build().await? {
        builder::ImportResult::Recipe(r) => Ok(r),
        builder::ImportResult::Cooklang(_) => unreachable!("extract_only sets Recipe mode"),
    }
}

/// Converts a recipe to Cooklang format with explicit configuration.
///
/// This is a simplified wrapper that uses the new pipeline architecture.
/// For more control, use `RecipeImporter::builder()` directly.
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
    // Convert recipe to text format
    let text = recipe.to_text_with_metadata();

    // Use builder with provider configuration
    let mut builder = RecipeImporter::builder().text(&text);

    if let Some(name) = provider_name {
        let provider = match name {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "google" => LlmProvider::Google,
            "ollama" => LlmProvider::Ollama,
            "azure_openai" => LlmProvider::AzureOpenAI,
            _ => {
                return Err(ImportError::ConversionError(format!(
                    "Unknown provider: {}",
                    name
                )))
            }
        };
        builder = builder.provider(provider);
    }

    if let Some(key) = api_key {
        builder = builder.api_key(&key);
    }

    if let Some(m) = model {
        builder = builder.model(&m);
    }

    match builder.build().await? {
        builder::ImportResult::Cooklang(s) => Ok(s),
        builder::ImportResult::Recipe(_) => unreachable!("Default mode is Cooklang"),
    }
}

/// Converts a recipe to Cooklang format using a custom provider.
///
/// This is a simplified wrapper that uses the new pipeline architecture.
/// For more control, use `RecipeImporter::builder()` directly.
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
    // Convert recipe to text format
    let text = recipe.to_text_with_metadata();

    // Use builder with provider name
    let mut builder = RecipeImporter::builder().text(&text);

    if let Some(name) = provider_name {
        let provider = match name {
            "openai" => LlmProvider::OpenAI,
            "anthropic" => LlmProvider::Anthropic,
            "google" => LlmProvider::Google,
            "ollama" => LlmProvider::Ollama,
            "azure_openai" => LlmProvider::AzureOpenAI,
            _ => {
                return Err(ImportError::ConversionError(format!(
                    "Unknown provider: {}",
                    name
                )))
            }
        };
        builder = builder.provider(provider);
    }

    match builder.build().await? {
        builder::ImportResult::Cooklang(s) => Ok(s),
        builder::ImportResult::Recipe(_) => unreachable!("Default mode is Cooklang"),
    }
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
/// A `Recipe` struct containing the recipe ingredients and instructions
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
///     println!("Ingredients: {:?}", recipe.ingredients);
///     println!("Instructions: {}", recipe.instructions);
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

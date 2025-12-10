pub mod builder;
pub mod config;
pub mod converters;
pub mod error;
pub mod images_to_text;
pub(crate) mod model;
pub mod pipelines;
pub mod url_to_text;

#[cfg(feature = "uniffi")]
pub mod uniffi_bindings;

// Re-export UniFFI types when feature is enabled
#[cfg(feature = "uniffi")]
pub use uniffi_bindings::*;

// Public API re-exports
pub use config::AiConfig;
pub use converters::{ConversionMetadata, ConversionResult, TokenUsage};
pub use error::ImportError;
pub use images_to_text::ImageSource;
pub use pipelines::RecipeComponents;

// Advanced builder API (for users who need more control)
pub use builder::{ImportResult, LlmProvider, RecipeImporter, RecipeImporterBuilder};

/// Extract recipe components from a URL.
///
/// Returns `RecipeComponents` with text, metadata, and name fields.
/// Empty strings are used for fields that couldn't be extracted.
///
/// # Arguments
/// * `url` - The URL of the recipe webpage to fetch
///
/// # Returns
/// * `Ok(RecipeComponents)` - The extracted recipe components
/// * `Err(ImportError)` - If the URL cannot be fetched or parsed
///
/// # Example
/// ```no_run
/// use cooklang_import::url_to_recipe;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let recipe = url_to_recipe("https://example.com/recipe").await?;
///     println!("Name: {}", recipe.name);
///     println!("Text: {}", recipe.text);
///     Ok(())
/// }
/// ```
pub async fn url_to_recipe(url: &str) -> Result<RecipeComponents, ImportError> {
    pipelines::url::process(url)
        .await
        .map_err(|e| ImportError::ExtractionError(e.to_string()))
}

/// Extract recipe components from images.
///
/// Returns `RecipeComponents` with text extracted via OCR.
/// Metadata contains the source info, name is typically empty.
///
/// Requires GOOGLE_API_KEY environment variable to be set.
///
/// # Arguments
/// * `images` - Vector of image sources (paths or base64-encoded data)
///
/// # Returns
/// * `Ok(RecipeComponents)` - The extracted recipe components
/// * `Err(ImportError)` - If OCR fails
///
/// # Example
/// ```no_run
/// use cooklang_import::{image_to_recipe, ImageSource};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let images = vec![ImageSource::Path("/path/to/recipe.jpg".to_string())];
///     let recipe = image_to_recipe(&images).await?;
///     println!("Text: {}", recipe.text);
///     Ok(())
/// }
/// ```
pub async fn image_to_recipe(images: &[ImageSource]) -> Result<RecipeComponents, ImportError> {
    pipelines::image::process(images)
        .await
        .map_err(|e| ImportError::ExtractionError(e.to_string()))
}

/// Parse text into recipe components.
///
/// If `extract` is true, uses LLM to extract structured recipe data.
/// If `extract` is false, parses the text assuming it's already formatted
/// with optional YAML frontmatter.
///
/// # Arguments
/// * `text` - The recipe text
/// * `extract` - Whether to use LLM extraction
///
/// # Returns
/// * `Ok(RecipeComponents)` - The parsed recipe components
/// * `Err(ImportError)` - If parsing or extraction fails
///
/// # Example
/// ```no_run
/// use cooklang_import::text_to_recipe;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let text = "2 eggs\n1 cup flour\n\nMix and bake at 350F.";
///     let recipe = text_to_recipe(text, false).await?;
///     println!("Text: {}", recipe.text);
///     Ok(())
/// }
/// ```
pub async fn text_to_recipe(text: &str, extract: bool) -> Result<RecipeComponents, ImportError> {
    pipelines::text::process(text, extract)
        .await
        .map_err(|e| ImportError::ExtractionError(e.to_string()))
}

/// Convert recipe text to Cooklang format.
///
/// Takes recipe text (ingredients + instructions) and converts it
/// to Cooklang format using an LLM. Returns the Cooklang text
/// with optional YAML frontmatter if metadata/name are provided.
///
/// # Arguments
/// * `components` - The recipe components to convert
///
/// # Returns
/// * `Ok(String)` - The recipe in Cooklang format
/// * `Err(ImportError)` - If conversion fails
///
/// # Example
/// ```no_run
/// use cooklang_import::{text_to_cooklang, RecipeComponents};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let components = RecipeComponents {
///         text: "2 eggs\n1 cup flour\n\nMix and bake at 350F.".to_string(),
///         metadata: String::new(),
///         name: "Simple Cake".to_string(),
///     };
///     let cooklang = text_to_cooklang(&components).await?;
///     println!("{}", cooklang);
///     Ok(())
/// }
/// ```
pub async fn text_to_cooklang(components: &RecipeComponents) -> Result<String, ImportError> {
    match RecipeImporter::builder()
        .text(&components.text)
        .build()
        .await?
    {
        ImportResult::Cooklang { mut content, .. } => {
            // Prepend frontmatter if we have name or metadata
            let has_name = !components.name.is_empty();
            let has_metadata = !components.metadata.is_empty();

            if has_name || has_metadata {
                let mut frontmatter = String::from("---\n");
                if has_name {
                    frontmatter.push_str(&format!("title: {}\n", components.name));
                }
                if has_metadata {
                    frontmatter.push_str(&components.metadata);
                    if !components.metadata.ends_with('\n') {
                        frontmatter.push('\n');
                    }
                }
                frontmatter.push_str("---\n\n");
                content = frontmatter + &content;
            }
            Ok(content)
        }
        ImportResult::Components(_) => unreachable!("Default mode is Cooklang"),
    }
}

use std::path::Path;
use std::time::Duration;

use crate::{
    convert_recipe_with_config, convert_recipe_with_provider, fetch_recipe_with_timeout,
    ocr::ocr_image_file, ImportError, Recipe,
};

/// Represents the input source for a recipe
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Fetch recipe from a URL
    Url(String),
    /// Use plain text content
    Text(String),
    /// Use image file (will be OCR'd using Google Vision)
    Image(String),
}

/// Represents the desired output format
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputMode {
    /// Convert to Cooklang format (default)
    #[default]
    Cooklang,
    /// Return Recipe struct without conversion
    Recipe,
}

/// Result of a recipe import operation
#[derive(Debug, Clone)]
pub enum ImportResult {
    /// Cooklang-formatted recipe
    Cooklang(String),
    /// Recipe struct (markdown format)
    Recipe(Recipe),
}

/// Optional LLM provider configuration
#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Google,
    AzureOpenAI,
    Ollama,
}

impl LlmProvider {
    /// Convert to provider name string used by the factory
    fn as_str(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::Google => "google",
            LlmProvider::AzureOpenAI => "azure_openai",
            LlmProvider::Ollama => "ollama",
        }
    }
}

/// Builder for configuring and executing recipe imports
#[derive(Debug, Default)]
pub struct RecipeImporterBuilder {
    source: Option<InputSource>,
    mode: OutputMode,
    provider: Option<LlmProvider>,
    timeout: Option<Duration>,
    api_key: Option<String>,
    model: Option<String>,
}

impl RecipeImporterBuilder {
    /// Set the input source to a URL
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe");
    /// ```
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.source = Some(InputSource::Url(url.into()));
        self
    }

    /// Set the input source to plain text
    ///
    /// Use this when you have a recipe in plain text format that needs to be parsed.
    /// The LLM will extract ingredients and instructions from the text.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let recipe_text = "Take 2 eggs and 1 cup of flour. Mix them together and bake at 350F for 30 minutes.";
    /// let builder = RecipeImporter::builder()
    ///     .text(recipe_text);
    /// ```
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.source = Some(InputSource::Text(text.into()));
        self
    }

    /// Set the input source to an image file
    ///
    /// Use this when you have a recipe image that needs to be OCR'd first.
    /// The image will be processed using Google Cloud Vision API to extract text,
    /// then the text will be converted to Cooklang format.
    ///
    /// Requires GOOGLE_API_KEY environment variable to be set.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .image("/path/to/recipe-image.jpg");
    /// ```
    pub fn image(mut self, image_path: impl Into<String>) -> Self {
        self.source = Some(InputSource::Image(image_path.into()));
        self
    }

    /// Set output mode to extract only (no conversion)
    ///
    /// This returns a Recipe struct without converting to Cooklang format.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .extract_only();
    /// ```
    pub fn extract_only(mut self) -> Self {
        self.mode = OutputMode::Recipe;
        self
    }

    /// Set a custom LLM provider for conversion
    ///
    /// # Example
    /// ```
    /// use cooklang_import::{RecipeImporter, LlmProvider};
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .provider(LlmProvider::Anthropic);
    /// ```
    pub fn provider(mut self, provider: LlmProvider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Set a timeout for HTTP requests
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    /// use std::time::Duration;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .timeout(Duration::from_secs(30));
    /// ```
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set the API key for the LLM provider
    ///
    /// This allows passing the API key directly instead of relying on
    /// environment variables or config files.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::{RecipeImporter, LlmProvider};
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .provider(LlmProvider::Anthropic)
    ///     .api_key("your-api-key");
    /// ```
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the model name for the LLM provider
    ///
    /// # Example
    /// ```
    /// use cooklang_import::{RecipeImporter, LlmProvider};
    ///
    /// let builder = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .provider(LlmProvider::Anthropic)
    ///     .model("claude-3-5-sonnet-20241022");
    /// ```
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Build and execute the recipe import operation
    ///
    /// # Returns
    /// An `ImportResult` containing either a Cooklang string or Recipe struct
    ///
    /// # Errors
    /// Returns `ImportError` if:
    /// - No input source was specified
    /// - URL fetch fails
    /// - Recipe extraction fails
    /// - Conversion fails
    /// - Invalid combination of options (e.g., markdown + extract_only)
    ///
    /// # Example
    /// ```no_run
    /// # use cooklang_import::RecipeImporter;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let result = RecipeImporter::builder()
    ///     .url("https://example.com/recipe")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> Result<ImportResult, ImportError> {
        // Validate that source is set
        let source = self.source.ok_or_else(|| {
            ImportError::BuilderError(
                "No input source specified. Use .url() or .markdown()".to_string(),
            )
        })?;

        // Convert provider enum to string name if provided
        let provider_name = self.provider.as_ref().map(|p| p.as_str());

        match (source, self.mode) {
            // Use Case 1: URL → Cooklang
            (InputSource::Url(url), OutputMode::Cooklang) => {
                let recipe = fetch_recipe_with_timeout(&url, self.timeout).await?;
                let cooklang = if self.api_key.is_some() || self.model.is_some() {
                    convert_recipe_with_config(&recipe, provider_name, self.api_key, self.model)
                        .await?
                } else {
                    convert_recipe_with_provider(&recipe, provider_name).await?
                };
                Ok(ImportResult::Cooklang(cooklang))
            }

            // Use Case 2: URL → Recipe (extract only)
            (InputSource::Url(url), OutputMode::Recipe) => {
                let recipe = fetch_recipe_with_timeout(&url, self.timeout).await?;
                Ok(ImportResult::Recipe(recipe))
            }

            // Use Case 3: Text → Cooklang
            (InputSource::Text(text), OutputMode::Cooklang) => {
                // Validate input
                if text.trim().is_empty() {
                    return Err(ImportError::InvalidMarkdown(
                        "Recipe text cannot be empty".to_string(),
                    ));
                }

                // Create Recipe struct
                let recipe = Recipe {
                    content: text,
                    ..Default::default()
                };

                let cooklang = if self.api_key.is_some() || self.model.is_some() {
                    convert_recipe_with_config(&recipe, provider_name, self.api_key, self.model)
                        .await?
                } else {
                    convert_recipe_with_provider(&recipe, provider_name).await?
                };
                Ok(ImportResult::Cooklang(cooklang))
            }

            // Invalid: Text → Recipe (no-op)
            (InputSource::Text { .. }, OutputMode::Recipe) => Err(ImportError::BuilderError(
                "Cannot use extract_only() with text input. Text needs to be parsed first."
                    .to_string(),
            )),

            // Use Case 4: Image → Cooklang (OCR then convert)
            (InputSource::Image(image_path), OutputMode::Cooklang) => {
                // Perform OCR on the image
                let text = ocr_image_file(Path::new(&image_path))
                    .await
                    .map_err(|e| {
                        ImportError::BuilderError(format!("Failed to OCR image: {}", e))
                    })?;

                // Validate OCR result
                if text.trim().is_empty() {
                    return Err(ImportError::BuilderError(
                        "No text detected in image".to_string(),
                    ));
                }

                // Create Recipe struct from OCR'd text
                let recipe = Recipe {
                    content: text,
                    ..Default::default()
                };

                // Convert to Cooklang
                let cooklang = if self.api_key.is_some() || self.model.is_some() {
                    convert_recipe_with_config(&recipe, provider_name, self.api_key, self.model)
                        .await?
                } else {
                    convert_recipe_with_provider(&recipe, provider_name).await?
                };
                Ok(ImportResult::Cooklang(cooklang))
            }

            // Invalid: Image → Recipe (OCR needs conversion)
            (InputSource::Image { .. }, OutputMode::Recipe) => Err(ImportError::BuilderError(
                "Cannot use extract_only() with image input. Images need to be OCR'd and parsed first."
                    .to_string(),
            )),
        }
    }
}

/// Main entry point for the builder API
pub struct RecipeImporter;

impl RecipeImporter {
    /// Creates a new builder for importing recipes
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder();
    /// ```
    pub fn builder() -> RecipeImporterBuilder {
        RecipeImporterBuilder::default()
    }
}

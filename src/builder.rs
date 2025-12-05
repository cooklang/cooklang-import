use std::time::Duration;

use crate::{images_to_text::ImageSource, ImportError, Recipe};

/// Represents the input source for a recipe
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Fetch recipe from a URL
    Url(String),
    /// Use text content (pre-formatted or requiring extraction)
    Text { content: String, extract: bool },
    /// Use images (paths or base64)
    Images(Vec<ImageSource>),
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

    /// Set the input source to pre-formatted text (no extraction needed)
    ///
    /// Use this when you have a recipe already formatted with ingredients and instructions.
    /// The text will be converted directly to Cooklang without LLM extraction.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let recipe_text = "2 eggs\n1 cup flour\n\nMix together and bake at 350F for 30 minutes.";
    /// let builder = RecipeImporter::builder()
    ///     .text(recipe_text);
    /// ```
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.source = Some(InputSource::Text {
            content: text.into(),
            extract: false,
        });
        self
    }

    /// Set the input source to plain text that needs extraction
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
    ///     .text_with_extraction(recipe_text);
    /// ```
    pub fn text_with_extraction(mut self, text: impl Into<String>) -> Self {
        self.source = Some(InputSource::Text {
            content: text.into(),
            extract: true,
        });
        self
    }

    /// Add an image file path to the input sources
    ///
    /// Use this when you have a recipe image that needs to be OCR'd.
    /// Multiple images can be added by calling this method multiple times.
    ///
    /// Requires GOOGLE_API_KEY environment variable to be set.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .image_path("/path/to/recipe-image.jpg");
    /// ```
    pub fn image_path(mut self, path: impl Into<String>) -> Self {
        match &mut self.source {
            Some(InputSource::Images(images)) => {
                images.push(ImageSource::Path(path.into()));
            }
            _ => {
                self.source = Some(InputSource::Images(vec![ImageSource::Path(path.into())]));
            }
        }
        self
    }

    /// Add a base64-encoded image to the input sources
    ///
    /// Use this when you have a recipe image as base64 data.
    /// Multiple images can be added by calling this method multiple times.
    ///
    /// Requires GOOGLE_API_KEY environment variable to be set.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::RecipeImporter;
    ///
    /// let builder = RecipeImporter::builder()
    ///     .image_base64("base64encodeddata...");
    /// ```
    pub fn image_base64(mut self, data: impl Into<String>) -> Self {
        match &mut self.source {
            Some(InputSource::Images(images)) => {
                images.push(ImageSource::Base64(data.into()));
            }
            _ => {
                self.source = Some(InputSource::Images(vec![ImageSource::Base64(data.into())]));
            }
        }
        self
    }

    /// Set multiple images at once
    ///
    /// Use this to set all images in one call instead of using image_path or image_base64 multiple times.
    ///
    /// # Example
    /// ```
    /// use cooklang_import::{RecipeImporter, ImageSource};
    ///
    /// let images = vec![
    ///     ImageSource::Path("/path/to/image1.jpg".to_string()),
    ///     ImageSource::Path("/path/to/image2.jpg".to_string()),
    /// ];
    /// let builder = RecipeImporter::builder()
    ///     .images(images);
    /// ```
    pub fn images(mut self, images: Vec<ImageSource>) -> Self {
        self.source = Some(InputSource::Images(images));
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
                "No input source specified. Use .url(), .text(), or .image_path()".to_string(),
            )
        })?;

        // Route to the appropriate pipeline based on input source
        let result = match source {
            InputSource::Url(url) => {
                crate::pipelines::url::process(&url)
                    .await
                    .map_err(|e| ImportError::BuilderError(e.to_string()))?
            }
            InputSource::Text { content, extract } => {
                crate::pipelines::text::process(&content, extract)
                    .await
                    .map_err(|e| ImportError::BuilderError(e.to_string()))?
            }
            InputSource::Images(images) => {
                crate::pipelines::image::process(&images)
                    .await
                    .map_err(|e| ImportError::BuilderError(e.to_string()))?
            }
        };

        // Return based on output mode
        match self.mode {
            OutputMode::Cooklang => Ok(ImportResult::Cooklang(result)),
            OutputMode::Recipe => {
                // Parse the result back into a Recipe struct
                let (metadata, body) = Recipe::parse_text_format(&result);
                let recipe = Recipe {
                    name: metadata.get("title").cloned().unwrap_or_default(),
                    description: metadata.get("description").cloned(),
                    metadata,
                    ingredients: vec![], // Pipelines return already formatted text
                    instructions: body,
                    ..Default::default()
                };
                Ok(ImportResult::Recipe(recipe))
            }
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

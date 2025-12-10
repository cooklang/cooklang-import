use std::time::Duration;

use crate::{
    config::{load_config, ProviderConfig},
    converters::{self, ConversionMetadata, Converter},
    images_to_text::ImageSource,
    pipelines::RecipeComponents,
    ImportError,
};

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
    /// Cooklang-formatted recipe with optional conversion metadata
    Cooklang {
        /// The converted Cooklang text
        content: String,
        /// Metadata about the LLM conversion (model, tokens, latency)
        conversion_metadata: Option<ConversionMetadata>,
    },
    /// Recipe components (text, metadata, name) - no conversion metadata since no LLM was used
    Components(RecipeComponents),
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
    // Note: Conversion to string is handled directly in the converter factory
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
        let source = self.source.clone().ok_or_else(|| {
            ImportError::BuilderError(
                "No input source specified. Use .url(), .text(), or .image_path()".to_string(),
            )
        })?;

        // Route to the appropriate pipeline based on input source
        let components = match source {
            InputSource::Url(url) => crate::pipelines::url::process(&url)
                .await
                .map_err(|e| ImportError::BuilderError(e.to_string()))?,
            InputSource::Text { content, extract } => {
                crate::pipelines::text::process(&content, extract)
                    .await
                    .map_err(|e| ImportError::BuilderError(e.to_string()))?
            }
            InputSource::Images(images) => crate::pipelines::image::process(&images)
                .await
                .map_err(|e| ImportError::BuilderError(e.to_string()))?,
        };

        // Return based on output mode
        match self.mode {
            OutputMode::Cooklang => {
                // Convert to Cooklang format using a converter
                let (content, conversion_metadata) =
                    self.convert_to_cooklang(&components).await?;
                Ok(ImportResult::Cooklang {
                    content,
                    conversion_metadata: Some(conversion_metadata),
                })
            }
            OutputMode::Recipe => Ok(ImportResult::Components(components)),
        }
    }

    /// Convert RecipeComponents to Cooklang using configured converter
    async fn convert_to_cooklang(
        &self,
        components: &RecipeComponents,
    ) -> Result<(String, ConversionMetadata), ImportError> {
        // Get converter configuration
        let converter = self.get_converter().await?;

        // Convert the text (ingredients + instructions) to Cooklang
        let conversion_result = converter
            .convert(&components.text)
            .await
            .map_err(|e| ImportError::ConversionError(e.to_string()))?;

        // Build YAML frontmatter from metadata and name
        let mut output = String::new();
        let has_name = !components.name.is_empty();
        let has_metadata = !components.metadata.is_empty();

        if has_name || has_metadata {
            output.push_str("---\n");
            if has_name {
                output.push_str(&format!("title: {}\n", components.name));
            }
            if has_metadata {
                output.push_str(&components.metadata);
                if !components.metadata.ends_with('\n') {
                    output.push('\n');
                }
            }
            output.push_str("---\n\n");
        }
        output.push_str(&conversion_result.content);

        Ok((output, conversion_result.metadata))
    }

    /// Get the appropriate converter based on configuration
    async fn get_converter(&self) -> Result<Box<dyn Converter>, ImportError> {
        // Determine which provider to use
        let provider_name: String = match &self.provider {
            Some(LlmProvider::OpenAI) => "open_ai".to_string(),
            Some(LlmProvider::Anthropic) => "anthropic".to_string(),
            Some(LlmProvider::Google) => "google".to_string(),
            Some(LlmProvider::AzureOpenAI) => "azure_openai".to_string(),
            Some(LlmProvider::Ollama) => "ollama".to_string(),
            None => {
                // Try to load from config, or default to open_ai
                load_config()
                    .map(|c| c.default_provider)
                    .unwrap_or_else(|_| "open_ai".to_string())
            }
        };

        // Build provider config
        let provider_config = self.build_provider_config(&provider_name);

        // Create the converter
        converters::create_converter(&provider_name, &provider_config).ok_or_else(|| {
            ImportError::ConversionError(format!(
                "Failed to create converter '{}'. Check API key and configuration.",
                provider_name
            ))
        })
    }

    /// Build provider configuration from builder settings and environment
    fn build_provider_config(&self, provider_name: &str) -> ProviderConfig {
        // Try to load config from file first
        let base_config = load_config()
            .ok()
            .and_then(|c| c.providers.get(provider_name).cloned());

        // Build config with overrides from builder
        ProviderConfig {
            enabled: true,
            model: self.model.clone().unwrap_or_else(|| {
                base_config
                    .as_ref()
                    .map(|c| c.model.clone())
                    .unwrap_or_else(|| default_model_for_provider(provider_name).to_string())
            }),
            temperature: base_config.as_ref().map(|c| c.temperature).unwrap_or(0.7),
            max_tokens: base_config.as_ref().map(|c| c.max_tokens).unwrap_or(4000),
            api_key: self
                .api_key
                .clone()
                .or_else(|| base_config.as_ref().and_then(|c| c.api_key.clone())),
            base_url: base_config.as_ref().and_then(|c| c.base_url.clone()),
            endpoint: base_config.as_ref().and_then(|c| c.endpoint.clone()),
            deployment_name: base_config.as_ref().and_then(|c| c.deployment_name.clone()),
            api_version: base_config.as_ref().and_then(|c| c.api_version.clone()),
            project_id: base_config.as_ref().and_then(|c| c.project_id.clone()),
        }
    }
}

/// Get default model for a given provider
fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "open_ai" => "gpt-4o-mini",
        "anthropic" => "claude-haiku-4-5",
        "google" => "gemini-1.5-flash",
        "azure_openai" => "gpt-4",
        "ollama" => "llama2",
        _ => "gpt-4o-mini",
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

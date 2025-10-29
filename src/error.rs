use thiserror::Error;

/// Errors that can occur during recipe import operations
#[derive(Error, Debug)]
pub enum ImportError {
    /// Failed to fetch recipe from URL
    #[error("Failed to fetch URL: {0}")]
    FetchError(#[from] reqwest::Error),

    /// Failed to parse recipe from webpage
    #[error("Failed to parse recipe: {0}")]
    ParseError(String),

    /// No extractor could successfully parse the recipe
    #[error("No extractor could parse the recipe from this webpage")]
    NoExtractorMatched,

    /// Failed to convert recipe to Cooklang format
    #[error("Conversion failed: {0}")]
    ConversionError(String),

    /// Invalid markdown format provided
    #[error("Invalid markdown format: {0}")]
    InvalidMarkdown(String),

    /// Builder configuration error
    #[error("Builder error: {0}")]
    BuilderError(String),

    /// Error parsing HTTP headers
    #[error("Header parse error: {0}")]
    HeaderError(#[from] reqwest::header::InvalidHeaderValue),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvError(#[from] std::env::VarError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),
}

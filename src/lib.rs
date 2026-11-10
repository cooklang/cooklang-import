#[cfg(feature = "native")]
mod api;
#[cfg(feature = "native")]
pub mod builder;
#[cfg(feature = "native")]
pub mod config;
#[cfg(feature = "native")]
pub mod converters;
#[cfg(feature = "native")]
pub mod error;
pub mod extract;
#[cfg(feature = "native")]
pub mod images_to_text;
pub mod model;
pub mod pipelines;
pub mod url_to_text;
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "uniffi")]
pub mod uniffi_bindings;

// Re-export UniFFI types when feature is enabled
#[cfg(feature = "uniffi")]
pub use uniffi_bindings::*;

// Public API re-exports
#[cfg(feature = "native")]
pub use api::{image_to_recipe, text_to_cooklang, text_to_recipe, url_to_recipe};
#[cfg(feature = "native")]
pub use config::AiConfig;
#[cfg(feature = "native")]
pub use converters::{
    inject_evaluation, ConversionMetadata, ConversionResult, TokenUsage, COOKLANG_EVALUATOR_PROMPT,
};
#[cfg(feature = "native")]
pub use error::ImportError;
#[cfg(feature = "native")]
pub use images_to_text::ImageSource;
pub use pipelines::RecipeComponents;

// Advanced builder API (for users who need more control)
#[cfg(feature = "native")]
pub use builder::{ImportResult, LlmProvider, RecipeImporter, RecipeImporterBuilder};

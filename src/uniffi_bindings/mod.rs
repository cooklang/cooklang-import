//! UniFFI bindings for cooklang-import
//!
//! FFI-compatible types and functions for iOS and Android. What is in this file needs
//! no network, no LLM and no async runtime, so it is all an extract-only build
//! (`--no-default-features --features uniffi`) contains: an app that already holds a
//! page's HTML, such as an in-app browser, links the structured extractors without
//! the HTTP and TLS stack. The exports that fetch or convert are in `native`.

use crate::RecipeComponents;

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;

uniffi::setup_scaffolding!();

/// FFI-compatible recipe components structure
#[derive(Debug, Clone, uniffi::Record)]
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

/// Extract recipe components from a page's HTML with the structured extractors
/// (JSON-LD, microdata, HTML classes).
///
/// Makes no network request and calls no LLM: the caller already has the page, for
/// example from a web view the user is signed in to. The components are the same
/// ones the URL pipeline produces after a successful structured extraction.
///
/// # Arguments
/// * `html` - The page's HTML
/// * `url` - The page's URL, used for the source metadata
///
/// # Returns
/// The extracted components, or `None` when no extractor finds a recipe with
/// instructions.
#[uniffi::export]
pub fn extract_recipe_from_html(html: String, url: String) -> Option<FfiRecipeComponents> {
    crate::extract::extract(&html, &url).map(|(_, components)| components.into())
}

/// Get the library version
#[uniffi::export]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"<html><head><script type="application/ld+json">
    {"@context":"https://schema.org","@type":"Recipe","name":"Pancakes","recipeYield":"4",
     "recipeIngredient":["200 g flour","2 eggs"],
     "recipeInstructions":[{"@type":"HowToStep","text":"Whisk the flour and eggs."},
                           {"@type":"HowToStep","text":"Fry in a hot pan."}]}
    </script></head><body></body></html>"#;

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

    // Metadata is compared by its lines: the extractor keeps it in a HashMap, so the
    // order of the YAML keys differs from one call to the next.
    #[test]
    fn extract_recipe_from_html_returns_the_extracted_components() {
        let url = "https://example.com/pancakes";
        let (_, expected) = crate::extract::extract(PAGE, url).unwrap();

        let components = extract_recipe_from_html(PAGE.to_string(), url.to_string()).unwrap();

        assert_eq!(components.name, "Pancakes");
        assert_eq!(components.text, expected.text);
        assert!(components.text.contains("Fry in a hot pan."));
        let mut lines: Vec<&str> = components.metadata.lines().collect();
        let mut expected_lines: Vec<&str> = expected.metadata.lines().collect();
        lines.sort_unstable();
        expected_lines.sort_unstable();
        assert_eq!(lines, expected_lines);
        assert!(lines.contains(&"servings: 4"));
        assert!(lines.contains(&"source: https://example.com/pancakes"));
    }

    #[test]
    fn extract_recipe_from_html_is_none_without_a_recipe() {
        assert!(extract_recipe_from_html(
            "<html><body>Nothing here</body></html>".to_string(),
            "https://example.com".to_string()
        )
        .is_none());
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
    }
}

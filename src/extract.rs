//! Network-free extraction: a page's HTML in, the recipe out.
//!
//! This is the half of the URL pipeline that needs no fetcher, no LLM and no async
//! runtime, so it also builds for `wasm32-unknown-unknown` (see `crate::wasm`). The
//! browser extension runs it on the page the user is looking at and sends the
//! resulting components to the server, which then skips its own extraction.

use crate::model::Recipe;
use crate::pipelines::{recipe_to_components, RecipeComponents};
use crate::url_to_text::html::extractors::extract_recipe;
use serde::Serialize;

#[derive(Serialize)]
struct ComponentsJson<'a> {
    name: &'a str,
    metadata: &'a str,
    text: &'a str,
}

#[derive(Serialize)]
struct ExtractionJson<'a> {
    recipe: &'a Recipe,
    components: ComponentsJson<'a>,
}

/// Extract a recipe from HTML with the structured extractors.
///
/// Returns the raw `Recipe` (for display) and the `RecipeComponents` the converter
/// takes: the same pair the URL pipeline produces after a successful structured
/// extraction. `None` when no extractor finds a recipe with instructions.
pub fn extract(html: &str, url: &str) -> Option<(Recipe, RecipeComponents)> {
    let recipe = extract_recipe(html, url)?;
    let components = recipe_to_components(&recipe);
    Some((recipe, components))
}

/// `extract`, serialized as
/// `{"recipe": {name, description, image, ingredients, instructions, metadata},
///   "components": {name, metadata, text}}`.
pub fn extract_json(html: &str, url: &str) -> Option<String> {
    let (recipe, components) = extract(html, url)?;
    serde_json::to_string(&ExtractionJson {
        recipe: &recipe,
        components: ComponentsJson {
            name: &components.name,
            metadata: &components.metadata,
            text: &components.text,
        },
    })
    .ok()
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
    fn extract_returns_recipe_and_components() {
        let (recipe, components) = extract(PAGE, "https://example.com/pancakes").unwrap();
        assert_eq!(recipe.name, "Pancakes");
        assert_eq!(components.name, "Pancakes");
        assert!(components.text.starts_with("200 g flour\n2 eggs\n\n"));
        assert!(components.text.contains("Fry in a hot pan."));
        assert!(components.metadata.contains("servings: 4"));
    }

    #[test]
    fn extract_json_has_both_halves() {
        let json = extract_json(PAGE, "https://example.com/pancakes").unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["recipe"]["name"], "Pancakes");
        assert_eq!(value["recipe"]["ingredients"][1], "2 eggs");
        assert!(value["recipe"]["metadata"].is_object());
        assert_eq!(value["components"]["name"], "Pancakes");
        assert!(value["components"]["text"]
            .as_str()
            .unwrap()
            .contains("Whisk"));
        assert!(value["components"]["metadata"]
            .as_str()
            .unwrap()
            .contains("servings: 4"));
    }

    #[test]
    fn extract_json_is_none_without_a_recipe() {
        assert!(extract_json(
            "<html><body>Nothing here</body></html>",
            "https://example.com"
        )
        .is_none());
    }
}

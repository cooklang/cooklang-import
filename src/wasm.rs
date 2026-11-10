//! The browser entry point. One export; the npm wrapper in `wasm/index.js` parses
//! the JSON so callers get a plain object.

use wasm_bindgen::prelude::*;

/// Extract a recipe from a page's HTML.
///
/// Returns the JSON described at `crate::extract::extract_json`, or `undefined`
/// when the page has no structured recipe.
#[wasm_bindgen(js_name = extractRecipeJson)]
pub fn extract_recipe_json(html: &str, url: &str) -> Option<String> {
    crate::extract::extract_json(html, url)
}

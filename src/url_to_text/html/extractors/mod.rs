use crate::model::Recipe;
use scraper::Html;

mod html_class;
mod json_ld;
mod microdata;

pub use html_class::HtmlClassExtractor;
pub use json_ld::JsonLdExtractor;
pub use microdata::MicroDataExtractor;

pub struct ParsingContext {
    pub url: String,
    pub document: Html,
    pub texts: Option<String>,
}

pub trait Extractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>>;
}

/// Run the structured extractors over a page in order (JSON-LD, MicroData, HTML
/// class) and return the first usable recipe.
///
/// A hit with no instructions is worse than no hit at all: it stops the pipeline
/// before the remaining extractors and the LLM fallback run, and the caller
/// converts a partial recipe. A page yielding ingredients and nothing else used to
/// be returned as a success, and the converter, handed a list of ingredients with
/// no method, answered "no recipe". Thirteen of the 55 failures in the
/// 2026-08-31..09-07 window carry that byte-identical message, papillesetpupilles.fr
/// (8 ingredients, 0 instructions) and chefkoch.de (16 and 0) among them. Both
/// pages do publish their method; the extractor missed it, and stopping there
/// denied the later extractors and the LLM their chance to find it.
///
/// No network, no async: this is the part of the URL pipeline that also runs in
/// the browser (see `crate::extract`).
pub fn extract_recipe(html: &str, url: &str) -> Option<Recipe> {
    let context = ParsingContext {
        url: url.to_string(),
        document: Html::parse_document(html),
        texts: None,
    };

    let extractors: [&dyn Extractor; 3] =
        [&JsonLdExtractor, &MicroDataExtractor, &HtmlClassExtractor];

    for extractor in extractors {
        if let Ok(recipe) = extractor.parse(&context) {
            if recipe.instructions.trim().is_empty() {
                continue;
            }
            return Some(recipe);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_LD_PAGE: &str = r#"<html><head><script type="application/ld+json">
    {"@context":"https://schema.org","@type":"Recipe","name":"Pancakes","recipeYield":"4",
     "recipeIngredient":["200 g flour","2 eggs"],
     "recipeInstructions":[{"@type":"HowToStep","text":"Whisk the flour and eggs."},
                           {"@type":"HowToStep","text":"Fry in a hot pan."}]}
    </script></head><body></body></html>"#;

    #[test]
    fn extract_recipe_returns_the_json_ld_recipe() {
        let recipe = extract_recipe(JSON_LD_PAGE, "https://example.com/pancakes").unwrap();
        assert_eq!(recipe.name, "Pancakes");
        assert_eq!(recipe.ingredients, vec!["200 g flour", "2 eggs"]);
        assert!(recipe.instructions.contains("Whisk the flour and eggs."));
        assert!(recipe.instructions.contains("Fry in a hot pan."));
    }

    #[test]
    fn extract_recipe_is_none_for_a_page_without_a_recipe() {
        let html = "<html><body><p>Opening hours: 9 to 5.</p></body></html>";
        assert!(extract_recipe(html, "https://example.com/about").is_none());
    }

    #[test]
    fn extract_recipe_skips_a_hit_with_no_instructions() {
        let html = r#"<html><head><script type="application/ld+json">
        {"@context":"https://schema.org","@type":"Recipe","name":"Stub",
         "recipeIngredient":["1 egg"]}
        </script></head><body></body></html>"#;
        assert!(extract_recipe(html, "https://example.com/stub").is_none());
    }
}

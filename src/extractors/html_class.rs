use crate::extractors::{Extractor, ParsingContext};
use crate::model::Recipe;
use log::debug;
use scraper::{Html, Selector};
use std::collections::HashMap;

pub struct HtmlClassExtractor;

struct ClassMatchers {
    exact: HashMap<&'static str, Vec<&'static str>>,
    fuzzy: HashMap<&'static str, Vec<&'static str>>,
}

impl ClassMatchers {
    fn new() -> Self {
        let mut exact = HashMap::new();
        let mut fuzzy = HashMap::new();

        // WordPress Recipe Card (WPRM)
        exact.insert(
            "title",
            vec![
                "wprm-recipe-name",
                "tasty-recipes-title",
                "mv-create-title",
                "recipe-name",
                "recipe-title",
                "recipecardname",
                "recipe-card-title",
                "recipe-header-title",
                "wprp-recipe-title",
                "recipe_name",
                "recipe-content-title",
                "simple-recipe-pro-recipe-title",
                "recipe-callout-title",
                "wpzoom-recipe-card-title",
                "recipe-card__title",
                "wpupg-recipe-name",
                "recipe-card-title",
                "recipe-title-name",
                "recipess-recipe-title",
            ],
        );

        exact.insert(
            "description",
            vec![
                "wprm-recipe-summary",
                "recipe-summary",
                "recipe-description",
                "mv-create-description",
                "tasty-recipes-description",
                "recipe-card-summary",
                "wpzoom-recipe-summary",
                "recipe-summary-text",
                "recipe-intro",
                "recipe_description",
                "simple-recipe-pro-recipe-description",
                "recipe-callout-summary",
                "wpupg-recipe-summary",
                "recipe-card-description",
            ],
        );

        exact.insert(
            "ingredients",
            vec![
                "wprm-recipe-ingredients-container",
                "wprm-recipe-ingredient",
                "tasty-recipes-ingredients",
                "mv-create-ingredients",
                "recipe-ingredients",
                "recipe-ingredient-list",
                "recipe-card-ingredients",
                "wpzoom-recipe-ingredients",
                "recipe-ingredients-section",
                "simple-recipe-pro-recipe-ingredients",
                "recipe-callout-ingredients",
                "wpupg-recipe-ingredients",
                "recipe-card-ingredient-list",
                "recipe_ingredients",
                "recipess-ingredients-list",
                "structured-ingredients",
                "mpprecipe-ingredients",
                "recipe-content-ingredients",
                "recipe-ingredient-group",
            ],
        );

        exact.insert(
            "instructions",
            vec![
                "wprm-recipe-instructions-container",
                "wprm-recipe-instruction",
                "tasty-recipes-instructions",
                "mv-create-instructions",
                "recipe-instructions",
                "recipe-instruction-list",
                "recipe-card-instructions",
                "wpzoom-recipe-instructions",
                "recipe-instructions-section",
                "simple-recipe-pro-recipe-instructions",
                "recipe-callout-instructions",
                "wpupg-recipe-instructions",
                "recipe-card-instruction-list",
                "recipe_instructions",
                "recipess-instructions-list",
                "structured-instructions",
                "mpprecipe-instructions",
                "recipe-content-instructions",
                "recipe-instruction-group",
                "directions",
                "recipe-directions",
            ],
        );

        exact.insert(
            "prep_time",
            vec![
                "wprm-recipe-prep-time",
                "recipe-prep-time",
                "prep-time",
                "tasty-recipes-prep-time",
                "mv-create-time-prep",
                "recipe-card-prep-time",
                "wpzoom-recipe-prep-time",
                "recipe-prep_time",
                "simple-recipe-pro-prep-time",
                "wpupg-recipe-prep-time",
                "recipe-time-prep",
            ],
        );

        exact.insert(
            "cook_time",
            vec![
                "wprm-recipe-cook-time",
                "recipe-cook-time",
                "cook-time",
                "tasty-recipes-cook-time",
                "mv-create-time-active",
                "recipe-card-cook-time",
                "wpzoom-recipe-cook-time",
                "recipe-cook_time",
                "simple-recipe-pro-cook-time",
                "wpupg-recipe-cook-time",
                "recipe-time-cook",
            ],
        );

        exact.insert(
            "total_time",
            vec![
                "wprm-recipe-total-time",
                "recipe-total-time",
                "total-time",
                "tasty-recipes-total-time",
                "mv-create-time-total",
                "recipe-card-total-time",
                "wpzoom-recipe-total-time",
                "recipe-total_time",
                "simple-recipe-pro-total-time",
                "wpupg-recipe-total-time",
                "recipe-time-total",
            ],
        );

        exact.insert(
            "servings",
            vec![
                "wprm-recipe-servings",
                "recipe-yield",
                "recipe-servings",
                "tasty-recipes-yield",
                "mv-create-yield",
                "recipe-card-servings",
                "wpzoom-recipe-servings",
                "recipe-yield-value",
                "simple-recipe-pro-servings",
                "wpupg-recipe-servings",
                "recipe-card-yield",
                "recipeyield",
            ],
        );

        exact.insert(
            "notes",
            vec![
                "wprm-recipe-notes",
                "recipe-notes",
                "tasty-recipes-notes",
                "mv-create-notes",
                "recipe-card-notes",
                "wpzoom-recipe-notes",
                "recipe-tips",
                "simple-recipe-pro-notes",
                "wpupg-recipe-notes",
                "recipe-card-tips",
                "recipe-footnotes",
            ],
        );

        // Fuzzy matchers for fallback
        fuzzy.insert("title", vec!["recipe", "title", "name", "heading"]);

        fuzzy.insert("ingredients", vec!["ingredient"]);

        fuzzy.insert(
            "instructions",
            vec!["instruction", "direction", "method", "step"],
        );

        fuzzy.insert("description", vec!["summary", "description", "intro"]);

        ClassMatchers { exact, fuzzy }
    }

    fn find_by_class(&self, document: &Html, field: &str) -> Option<String> {
        // Try exact matches first
        if let Some(classes) = self.exact.get(field) {
            for class_name in classes {
                let selector_str = format!(".{class_name}");
                if let Ok(selector) = Selector::parse(&selector_str) {
                    let elements: Vec<_> = document.select(&selector).collect();
                    if !elements.is_empty() {
                        let text = elements
                            .iter()
                            .map(|el| el.text().collect::<Vec<_>>().join(" "))
                            .collect::<Vec<_>>()
                            .join("\n")
                            .trim()
                            .to_string();
                        if !text.is_empty() {
                            debug!("Found {} using exact class: {}", field, class_name);
                            return Some(text);
                        }
                    }
                };
            }
        }

        // Try fuzzy matches as fallback
        if let Some(patterns) = self.fuzzy.get(field) {
            for pattern in patterns {
                let selector_str = format!("[class*='{pattern}']");
                if let Ok(selector) = Selector::parse(&selector_str) {
                    let elements: Vec<_> = document.select(&selector).collect();
                    if !elements.is_empty() {
                        let text = elements
                            .iter()
                            .map(|el| el.text().collect::<Vec<_>>().join(" "))
                            .collect::<Vec<_>>()
                            .join("\n")
                            .trim()
                            .to_string();
                        if !text.is_empty() && text.len() < 5000 {
                            // Avoid grabbing entire page
                            debug!("Found {} using fuzzy class pattern: {}", field, pattern);
                            return Some(text);
                        }
                    }
                };
            }
        }

        None
    }

    fn extract_list_items(&self, document: &Html, field: &str) -> Vec<String> {
        let mut items = Vec::new();

        // Try to find container first
        if let Some(classes) = self.exact.get(field) {
            for class_name in classes {
                let selector_str = format!(".{class_name}");
                if let Ok(selector) = Selector::parse(&selector_str) {
                    for container in document.select(&selector) {
                        // Look for list items within container
                        if let Ok(li_selector) = Selector::parse("li") {
                            for li in container.select(&li_selector) {
                                let text =
                                    li.text().collect::<Vec<_>>().join(" ").trim().to_string();
                                if !text.is_empty() {
                                    items.push(text);
                                }
                            }
                        }

                        // If no list items, try div or p elements
                        if items.is_empty() {
                            for selector_str in &["div", "p", "span"] {
                                if let Ok(item_selector) = Selector::parse(selector_str) {
                                    for item in container.select(&item_selector) {
                                        let text = item
                                            .text()
                                            .collect::<Vec<_>>()
                                            .join(" ")
                                            .trim()
                                            .to_string();
                                        if !text.is_empty() && text.len() > 5 && text.len() < 500 {
                                            items.push(text);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !items.is_empty() {
                        debug!(
                            "Found {} {} using class: {}",
                            items.len(),
                            field,
                            class_name
                        );
                        return items;
                    }
                };
            }
        }

        items
    }
}

impl Extractor for HtmlClassExtractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>> {
        debug!("Attempting to extract recipe using HTML class matchers");

        let matchers = ClassMatchers::new();
        let mut metadata = HashMap::new();
        let mut name = String::new();
        let mut description = None;

        // Extract title
        if let Some(title) = matchers.find_by_class(&context.document, "title") {
            name = title;
        } else {
            // Try h1 or h2 as fallback
            if let Ok(selector) = Selector::parse("h1, h2") {
                if let Some(element) = context.document.select(&selector).next() {
                    name = element
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();
                }
            }
        }

        // Extract description
        if let Some(desc) = matchers.find_by_class(&context.document, "description") {
            description = Some(desc);
        }

        // Extract ingredients
        let ingredients_list = matchers.extract_list_items(&context.document, "ingredients");

        // Extract instructions
        let instructions_list = matchers.extract_list_items(&context.document, "instructions");

        // Extract metadata
        if let Some(prep_time) = matchers.find_by_class(&context.document, "prep_time") {
            metadata.insert("prep_time".to_string(), prep_time);
        }

        if let Some(cook_time) = matchers.find_by_class(&context.document, "cook_time") {
            metadata.insert("cook_time".to_string(), cook_time);
        }

        if let Some(total_time) = matchers.find_by_class(&context.document, "total_time") {
            metadata.insert("total_time".to_string(), total_time);
        }

        if let Some(servings) = matchers.find_by_class(&context.document, "servings") {
            metadata.insert("servings".to_string(), servings);
        }

        if let Some(notes) = matchers.find_by_class(&context.document, "notes") {
            metadata.insert("notes".to_string(), notes);
        }

        // Add source URL to metadata
        metadata.insert("source_url".to_string(), context.url.clone());

        // Validation
        if name.is_empty() {
            return Err("Could not extract recipe title from HTML".into());
        }

        if ingredients_list.is_empty() && instructions_list.is_empty() {
            return Err("Could not extract recipe content from HTML".into());
        }

        // Convert lists to formatted strings
        let ingredients_str = if !ingredients_list.is_empty() {
            ingredients_list.join("\n")
        } else {
            String::new()
        };

        let instructions_str = if !instructions_list.is_empty() {
            instructions_list
                .iter()
                .enumerate()
                .map(|(i, instruction)| format!("{}. {}", i + 1, instruction))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        debug!("Successfully extracted recipe using HTML class matchers");
        debug!("Recipe name: {}", name);
        debug!("Ingredients count: {}", ingredients_list.len());
        debug!("Instructions count: {}", instructions_list.len());

        Ok(Recipe {
            name,
            description,
            image: Vec::new(),
            ingredients: ingredients_str,
            instructions: instructions_str,
            metadata,
        })
    }
}

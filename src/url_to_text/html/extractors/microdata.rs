use super::{Extractor, ParsingContext};
use crate::model::Recipe;
use log::debug;
use scraper::{ElementRef, Selector};
use std::collections::HashMap;

pub struct MicroDataExtractor;

impl MicroDataExtractor {
    fn find_recipe_container<'a>(&self, document: &'a scraper::Html) -> Option<ElementRef<'a>> {
        // Look for elements with itemscope and itemtype containing "Recipe"
        let selector = Selector::parse("[itemscope]").unwrap();
        for element in document.select(&selector) {
            if let Some(itemtype) = element.value().attr("itemtype") {
                if itemtype.contains("schema.org/Recipe")
                    || itemtype.contains("data-vocabulary.org/Recipe")
                {
                    return Some(element);
                }
            }
        }
        None
    }

    fn get_itemprop(&self, root: ElementRef, prop: &str) -> Option<String> {
        let selector = Selector::parse(&format!("[itemprop='{}']", prop)).unwrap();
        root.select(&selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
    }

    fn get_itemprop_list(&self, root: ElementRef, prop: &str) -> Vec<String> {
        let mut items = Vec::new();
        let selector = Selector::parse(&format!("[itemprop='{}']", prop)).unwrap();
        for el in root.select(&selector) {
            let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !text.is_empty() {
                items.push(text);
            }
        }
        items
    }
}

impl Extractor for MicroDataExtractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>> {
        debug!("Attempting to extract recipe using MicroData extractor");

        let container = self.find_recipe_container(&context.document);

        // We strictly enforce finding a Recipe container to avoid false positives.
        // Global searches for 'itemprop' (like "name" or "description") often pick up
        // unrelated page content (site title, author bio, ads, etc.) if not scoped
        // to a specific Schema.org Recipe item.
        if container.is_none() {
            return Err("No MicroData Recipe container found".into());
        }
        let container = container.unwrap();

        let mut metadata = HashMap::new();
        let name;
        let mut description = None;

        // Name
        if let Some(n) = self.get_itemprop(container, "name") {
            name = n;
        } else {
            return Err("Could not extract recipe name".into());
        }

        // Description
        if let Some(desc) = self.get_itemprop(container, "description") {
            description = Some(desc);
        }

        // Image
        // Try 'image' itemprop, check src attribute if it's an img tag
        let image_selector = Selector::parse("[itemprop='image']").unwrap();
        if let Some(img_el) = container.select(&image_selector).next() {
            if let Some(src) = img_el.value().attr("src") {
                metadata.insert("image".to_string(), src.to_string());
            } else {
                let text = img_el
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    metadata.insert("image".to_string(), text);
                }
            }
        }

        // Author
        // Author can be a string or a Person object
        let author_selector = Selector::parse("[itemprop='author']").unwrap();
        if let Some(author_el) = container.select(&author_selector).next() {
            // Check if it has nested name, otherwise use the author element itself
            let name_selector = Selector::parse("[itemprop='name']").unwrap();
            let target_el = author_el.select(&name_selector).next().unwrap_or(author_el);

            let text = target_el
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            if !text.is_empty() {
                metadata.insert("author".to_string(), text);
            }
        }

        // Times
        if let Some(prep) = self.get_itemprop(container, "prepTime") {
            metadata.insert("prep_time".to_string(), prep);
        }
        if let Some(cook) = self.get_itemprop(container, "cookTime") {
            metadata.insert("cook_time".to_string(), cook);
        }
        if let Some(total) = self.get_itemprop(container, "totalTime") {
            metadata.insert("total_time".to_string(), total);
        }

        // Yield/Servings
        if let Some(yield_val) = self.get_itemprop(container, "recipeYield") {
            metadata.insert("servings".to_string(), yield_val);
        }

        // Course / Category
        if let Some(category) = self.get_itemprop(container, "recipeCategory") {
            metadata.insert("course".to_string(), category);
        }

        // Cuisine
        if let Some(cuisine) = self.get_itemprop(container, "recipeCuisine") {
            metadata.insert("cuisine".to_string(), cuisine);
        }

        // Diet
        if let Some(diet) = self.get_itemprop(container, "suitableForDiet") {
            metadata.insert("diet".to_string(), diet);
        }

        // Keywords / Tags
        if let Some(keywords) = self.get_itemprop(container, "keywords") {
            metadata.insert("tags".to_string(), keywords);
        }

        // Ingredients
        // Try 'ingredients' and 'recipeIngredients'
        let mut ingredients = self.get_itemprop_list(container, "ingredients");
        if ingredients.is_empty() {
            ingredients = self.get_itemprop_list(container, "recipeIngredient");
        }

        // Instructions
        // Try 'recipeInstructions' and 'instructions'
        let mut instructions_list = self.get_itemprop_list(container, "recipeInstructions");
        if instructions_list.is_empty() {
            instructions_list = self.get_itemprop_list(container, "instructions");
        }

        // Validation
        if ingredients.is_empty() && instructions_list.is_empty() {
            return Err("Could not extract recipe content".into());
        }

        // Combine instructions into a single string with paragraph breaks
        let instructions = instructions_list.join("\n\n");

        // Add source URL
        metadata.insert("source_url".to_string(), context.url.clone());

        Ok(Recipe {
            name,
            description,
            image: Vec::new(),
            ingredients,
            instructions,
            metadata,
        })
    }
}

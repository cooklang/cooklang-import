use crate::extractors::Extractor;
use crate::model::Recipe;
use html_escape::decode_html_entities;
use log::debug;
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;
use std::convert::TryFrom;
pub struct JsonLdExtractor;

#[derive(Debug, Deserialize)]
struct JsonLdRecipe {
    name: String,
    description: DescriptionType,
    image: ImageType,
    #[serde(rename = "recipeIngredient")]
    recipe_ingredient: Vec<String>,
    #[serde(rename = "recipeInstructions")]
    recipe_instructions: RecipeInstructions,
    // #[serde(rename = "recipeYield")]
    // recipe_yield: Option<RecipeYield>,
    // #[serde(rename = "prepTime")]
    // prep_time: Option<String>,
    // #[serde(rename = "cookTime")]
    // cook_time: Option<String>,
    // #[serde(rename = "totalTime")]
    // total_time: Option<String>,
    // #[serde(rename = "suitableForDiet")]
    // suitable_for_diet: Option<Vec<String>>,
    // #[serde(rename = "recipeCategory")]
    // recipe_category: Option<String>,
    // #[serde(rename = "recipeCuisine")]
    // recipe_cuisine: Option<String>,
    // keywords: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageObject {
    url: String,
}

#[derive(Debug, Deserialize)]
struct TextObject {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DescriptionType {
    String(String),
    Object(TextObject),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImageType {
    None,
    String(String),
    Object(ImageObject),
    // potentially multiple images as objects
    MultipleStrings(Vec<String>),
    MultipleObjects(Vec<ImageObject>),
}

#[derive(Debug, Deserialize)]
struct RecipeInstructionObject {
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RecipeInstructions {
    String(String),
    Multiple(Vec<String>),
    MultipleObject(Vec<RecipeInstructionObject>),
    HowTo(Vec<HowTo>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "@type")]
enum HowTo {
    HowToStep(HowToStep),
    HowToSection(HowToSection),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "@type")]
struct HowToStep {
    text: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "@type")]
struct HowToSection {
    #[serde(rename = "itemListElement")]
    item_list_element: Vec<HowToStep>,
}

impl TryFrom<Value> for JsonLdRecipe {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

fn decode_html_symbols(text: &str) -> String {
    // for some reason need to decode twice to get the correct string
    decode_html_entities(&decode_html_entities(text)).into_owned()
}

impl From<JsonLdRecipe> for Recipe {
    fn from(json_ld_recipe: JsonLdRecipe) -> Self {
        Recipe {
            name: decode_html_symbols(&json_ld_recipe.name),
            description: match json_ld_recipe.description {
                DescriptionType::String(desc) => decode_html_symbols(&desc),
                DescriptionType::Object(desc) => decode_html_symbols(&desc.text),
            },
            image: match json_ld_recipe.image {
                ImageType::String(img) => vec![decode_html_symbols(&img)],
                ImageType::MultipleStrings(imgs) => imgs
                    .into_iter()
                    .map(|img| decode_html_symbols(&img))
                    .collect(),
                ImageType::MultipleObjects(imgs) => imgs.into_iter().map(|img| img.url).collect(),
                ImageType::None => vec![],
                ImageType::Object(img) => vec![img.url],
            },
            ingredients: json_ld_recipe
                .recipe_ingredient
                .into_iter()
                .map(|ing| decode_html_symbols(&ing))
                .collect(),
            steps: match json_ld_recipe.recipe_instructions {
                RecipeInstructions::String(instructions) => decode_html_symbols(&instructions),
                RecipeInstructions::Multiple(instructions) => instructions
                    .into_iter()
                    .map(|step| decode_html_symbols(&step))
                    .collect::<Vec<String>>()
                    .join(" "),
                RecipeInstructions::MultipleObject(instructions) => instructions
                    .iter()
                    .map(|obj| decode_html_symbols(&obj.text))
                    .collect::<Vec<String>>()
                    .join(" "),
                RecipeInstructions::HowTo(sections) => sections
                    .into_iter()
                    .flat_map(|section| match section {
                        HowTo::HowToStep(step) => {
                            let mut texts = Vec::new();
                            if let Some(text) = step.text {
                                texts.push(text);
                            }
                            if let Some(desc) = step.description {
                                texts.push(desc);
                            }
                            texts
                        }
                        HowTo::HowToSection(section) => section
                            .item_list_element
                            .into_iter()
                            .flat_map(|step| {
                                let mut texts = Vec::new();
                                if let Some(text) = step.text {
                                    texts.push(text);
                                }
                                if let Some(desc) = step.description {
                                    texts.push(desc);
                                }
                                texts
                            })
                            .collect(),
                    })
                    .map(|text| decode_html_symbols(&text))
                    .collect::<Vec<String>>()
                    .join(" "),
            },
        }
    }
}

// Add this new function to clean JSON strings
fn sanitize_json(json_str: &str) -> String {
    // Remove any leading/trailing whitespace
    let mut cleaned = json_str.trim().to_string();

    // Handle cases where there might be multiple JSON objects
    if !cleaned.starts_with('{') && !cleaned.starts_with('[') {
        if let Some(start) = cleaned.find('{') {
            cleaned = cleaned[start..].to_string();
        }
    }

    // Remove any trailing comma followed by closing brace/bracket
    cleaned = cleaned.replace(",]", "]").replace(",}", "}");

    // Remove any HTML comments that might be present
    cleaned = cleaned.replace(r"<!--", "").replace("-->", "");

    cleaned
}

impl Extractor for JsonLdExtractor {
    fn can_parse(&self, document: &Html) -> bool {
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();

        debug!("Document: {}", document.html());

        // Check all script elements
        document.select(&selector).any(|script| {
            debug!("Script: {}", script.inner_html());

            // Use the sanitize function before parsing
            let cleaned_json = sanitize_json(&script.inner_html());
            if let Ok(json_ld) = serde_json::from_str::<Value>(&cleaned_json) {
                debug!("JSON-LD: {:#?}", json_ld);

                if json_ld.is_array() {
                    json_ld
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .find(|item| item.get("recipeInstructions").is_some())
                        })
                        .is_some()
                } else if json_ld.get("recipeInstructions").is_some() {
                    debug!(
                        "Recipe Instructions: {:#?}",
                        json_ld.get("recipeInstructions")
                    );
                    true
                } else if let Some(graph) = json_ld.get("@graph") {
                    debug!("Graph: {:#?}", graph);
                    graph
                        .as_array()
                        .and_then(|arr| {
                            arr.iter().find(|item| {
                                item.get("@type") == Some(&Value::String("Recipe".to_string()))
                            })
                        })
                        .is_some()
                } else {
                    debug!("No valid recipe found in JSON-LD");
                    false
                }
            } else {
                false
            }
        })
    }

    fn parse(&self, document: &Html) -> Result<Recipe, Box<dyn std::error::Error>> {
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();

        // Try each script element until we find a valid recipe
        for script in document.select(&selector) {
            // Use the sanitize function before parsing
            let cleaned_json = sanitize_json(&script.inner_html());
            if let Ok(json_ld) = serde_json::from_str::<Value>(&cleaned_json) {
                debug!("Trying JSON-LD: {:#?}", json_ld);

                let recipe_result: Option<JsonLdRecipe> = if json_ld.is_array() {
                    json_ld
                        .as_array()
                        .and_then(|arr| {
                            arr.iter()
                                .find(|item| item.get("recipeInstructions").is_some())
                        })
                        .and_then(|recipe| recipe.clone().try_into().ok())
                } else if json_ld.get("recipeInstructions").is_some() {
                    debug!(
                        "Recipe Instructions: {:#?}",
                        json_ld.get("recipeInstructions")
                    );
                    json_ld.try_into().ok()
                } else if let Some(graph) = json_ld.get("@graph") {
                    graph
                        .as_array()
                        .and_then(|arr| {
                            arr.iter().find(|item| {
                                item.get("@type") == Some(&Value::String("Recipe".to_string()))
                            })
                        })
                        .and_then(|recipe| recipe.clone().try_into().ok())
                } else {
                    None
                };

                // If we found a valid recipe, return it
                if let Some(recipe) = recipe_result {
                    debug!("Found valid recipe: {:#?}", recipe);
                    return Ok(Recipe::from(recipe));
                }
                // Otherwise continue to the next script block
            }
        }

        Err("No valid recipe found in any JSON-LD script".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    // Add helper function for tests
    fn create_html_document(json_ld: &str) -> Html {
        let html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                    {}
                </script>
            </head>
            <body></body>
            </html>
            "#,
            json_ld
        );
        Html::parse_document(&html)
    }

    #[test]
    fn test_can_parse() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org/",
                    "@type": "Recipe",
                    "name": "Test Recipe",
                    "recipeIngredient": ["ingredient 1", "ingredient 2"],
                    "recipeInstructions": ["step 1", "step 2"]
                }
                </script>
            </head>
            <body></body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let extractor = JsonLdExtractor;
        assert!(extractor.can_parse(&document));
    }

    #[test]
    fn test_parse_basic_recipe() {
        let extractor = JsonLdExtractor;
        let json_ld = r#"
        {
            "@context": "https://schema.org/",
            "@type": "Recipe",
            "name": "Chocolate Chip Cookies",
            "description": "Delicious homemade cookies",
            "image": "https://example.com/cookie.jpg",
            "recipeIngredient": ["flour", "sugar", "chocolate chips"],
            "recipeInstructions": "Mix ingredients. Bake at 350F for 10 minutes."
        }
        "#;
        let document = create_html_document(json_ld);

        let result = extractor.parse(&document).unwrap();

        assert_eq!(result.name, "Chocolate Chip Cookies");
        assert_eq!(result.description, "Delicious homemade cookies");
        assert_eq!(result.image, vec!["https://example.com/cookie.jpg"]);
        assert_eq!(
            result.ingredients,
            vec!["flour", "sugar", "chocolate chips"]
        );
        assert_eq!(
            result.steps,
            "Mix ingredients. Bake at 350F for 10 minutes."
        );
    }

    #[test]
    fn test_parse_recipe_with_array() {
        let extractor = JsonLdExtractor;
        let json_ld = r#"
        [
            {
                "@context": "https://schema.org/",
                "@type": "Recipe",
                "name": "Pasta Carbonara",
                "description": "Classic Italian pasta dish",
                "image": ["https://example.com/carbonara1.jpg", "https://example.com/carbonara2.jpg"],
                "recipeIngredient": ["spaghetti", "eggs", "bacon", "cheese"],
                "recipeInstructions": [
                    {"@type": "HowToStep", "text": "Cook pasta"},
                    {"@type": "HowToStep", "text": "Fry bacon"},
                    {"@type": "HowToStep", "text": "Mix eggs and cheese"},
                    {"@type": "HowToStep", "text": "Combine all ingredients"}
                ]
            },
            {
                "@type": "WebSite",
                "name": "Recipe Website"
            }
        ]
        "#;
        let document = create_html_document(json_ld);

        let result = extractor.parse(&document).unwrap();

        assert_eq!(result.name, "Pasta Carbonara");
        assert_eq!(result.description, "Classic Italian pasta dish");
        assert_eq!(
            result.image,
            vec![
                "https://example.com/carbonara1.jpg",
                "https://example.com/carbonara2.jpg"
            ]
        );
        assert_eq!(
            result.ingredients,
            vec!["spaghetti", "eggs", "bacon", "cheese"]
        );
        assert_eq!(
            result.steps,
            "Cook pasta Fry bacon Mix eggs and cheese Combine all ingredients"
        );
    }
}

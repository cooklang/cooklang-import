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
    #[serde(rename = "recipeIngredient", alias = "ingredients")]
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

impl TryFrom<Option<&Value>> for JsonLdRecipe {
    type Error = serde_json::Error;

    fn try_from(value: Option<&Value>) -> Result<Self, Self::Error> {
        serde_json::from_value(value.unwrap().clone())
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

impl Extractor for JsonLdExtractor {
    fn can_parse(&self, document: &Html) -> bool {
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();

        debug!("Document: {}", document.html());

        // Check all script elements
        document.select(&selector).any(|script| {
            debug!("Script: {}", script.inner_html());

            // Use the sanitize function before parsing
            let cleaned_json = sanitize_json(&script.inner_html());
            debug!("Cleaned JSON: {}", cleaned_json);
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
                    let extracted_recipe = json_ld.as_array().and_then(|arr| {
                        arr.iter()
                            .find(|item| item.get("recipeInstructions").is_some())
                    });

                    match JsonLdRecipe::try_from(extracted_recipe) {
                        Ok(recipe) => Some(recipe),
                        Err(e) => {
                            debug!("Failed to parse recipe: {}", e);
                            None
                        }
                    }
                } else if json_ld.get("recipeInstructions").is_some() {
                    debug!(
                        "Recipe Instructions: {:#?}",
                        json_ld.get("recipeInstructions")
                    );

                    match JsonLdRecipe::try_from(Some(&json_ld)) {
                        Ok(recipe) => Some(recipe),
                        Err(e) => {
                            debug!("Failed to parse recipe: {}", e);
                            None
                        }
                    }
                } else if let Some(graph) = json_ld.get("@graph") {
                    debug!("Graph: {:#?}", graph);
                    let extracted_recipe = graph.as_array().and_then(|arr| {
                        arr.iter().find(|item| {
                            item.get("@type") == Some(&Value::String("Recipe".to_string()))
                        })
                    });

                    match JsonLdRecipe::try_from(extracted_recipe) {
                        Ok(recipe) => Some(recipe),
                        Err(e) => {
                            debug!("Failed to parse recipe: {}", e);
                            None
                        }
                    }
                } else {
                    debug!("None of the conditions were met");
                    None
                };

                debug!("Recipe Result: {:#?}", recipe_result);

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

fn sanitize_json(json_str: &str) -> String {
    debug!("Original JSON: {}", json_str);

    let mut minified = String::with_capacity(json_str.len());
    let mut in_string = false;
    let mut prev_char = None;
    let mut depth = 0;
    let chars: Vec<char> = json_str.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        match c {
            '"' if prev_char != Some('\\') => {
                in_string = !in_string;
                if !in_string {
                    // We're ending a string - check if we need a comma
                    let rest_chars = &chars[i + 1..];
                    let next_char = rest_chars.iter().find(|c| !c.is_whitespace());
                    if !matches!(prev_char, Some(',') | Some('[') | Some('{'))
                        && matches!(next_char, Some('"' | '[' | '{'))
                    {
                        debug!("Adding missing comma after string");
                        minified.push('"');
                        minified.push(',');
                        prev_char = Some(',');
                        continue;
                    }
                }
                minified.push(c);
            }
            '[' | '{' if !in_string => {
                depth += 1;
                minified.push(c);
            }
            ']' | '}' if !in_string => {
                depth -= 1;
                minified.push(c);
                // Check if we need a comma after array/object closing
                let rest = &json_str[i + 1..];
                let next_char = rest.trim_start().chars().next();
                if depth > 0 && matches!(next_char, Some('"')) {
                    debug!("Adding missing comma after array/object closing");
                    minified.push(',');
                    prev_char = Some(',');
                    continue;
                }
            }
            ',' if !in_string => {
                // Avoid duplicate commas
                if prev_char != Some(',') {
                    minified.push(c);
                }
            }
            ':' if !in_string => {
                // Handle malformed key-value pairs
                if prev_char == Some(',') {
                    minified.pop(); // Remove the extra comma
                }
                minified.push(c);
            }
            _ => {
                if in_string || !c.is_whitespace() {
                    minified.push(c);
                }
            }
        }
        prev_char = Some(c);
    }

    // Clean up any remaining issues
    let cleaned = minified
        .replace(",]", "]")
        .replace(",}", "}")
        .replace(",,", ",")
        .replace(",:,", ":")
        .replace(":,", ":")
        .replace(",:", ":");

    debug!("Sanitized JSON: {}", cleaned);
    cleaned
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

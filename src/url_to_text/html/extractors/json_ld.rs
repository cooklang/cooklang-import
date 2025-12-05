use super::{Extractor, ParsingContext};
use crate::model::Recipe;
use html_escape::decode_html_entities;
use log::debug;
use scraper::Selector;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::TryFrom;

pub struct JsonLdExtractor;

impl JsonLdExtractor {
    fn convert_to_recipe(&self, json_ld_recipe: JsonLdRecipe, url: &str) -> Recipe {
        let mut metadata = HashMap::new();

        // Add source URL (primary key: source)
        metadata.insert("source".to_string(), url.to_string());

        // Map author
        if let Some(author) = json_ld_recipe.author {
            let author_name = match author {
                Author::String(name) => Some(name),
                Author::Object(obj) => obj.name,
                Author::Multiple(authors) => {
                    let names: Vec<String> = authors.into_iter().filter_map(|a| a.name).collect();
                    if names.is_empty() {
                        None
                    } else {
                        Some(names.join(", "))
                    }
                }
            };
            if let Some(name) = author_name {
                if !name.is_empty() {
                    metadata.insert("author".to_string(), name);
                }
            }
        }

        // Map servings (primary key according to Cooklang conventions)
        if let Some(yield_val) = json_ld_recipe.recipe_yield {
            let yield_str = match yield_val {
                RecipeYield::String(s) => s,
                RecipeYield::Number(n) => n.to_string(),
                RecipeYield::Array(arr) => {
                    // For arrays, prefer the descriptive version (e.g., "15 Stück") over just the number
                    arr.iter()
                        .find(|s| s.contains(char::is_alphabetic))
                        .or_else(|| arr.first())
                        .cloned()
                        .unwrap_or_default()
                }
            };
            if !yield_str.is_empty() {
                metadata.insert("servings".to_string(), yield_str);
            }
        }

        // Map course (primary key according to Cooklang conventions)
        if let Some(category) = json_ld_recipe.recipe_category {
            let category_str = match category {
                RecipeCategory::String(s) => s,
                RecipeCategory::Multiple(v) => v.join(", "),
            };
            if !category_str.is_empty() {
                metadata.insert("course".to_string(), category_str);
            }
        }

        // Map time fields (use specific keys, not duplicates)
        if let Some(total_time) = json_ld_recipe.total_time {
            if !total_time.is_empty() {
                metadata.insert("time required".to_string(), convert_duration(&total_time));
            }
        }

        if let Some(prep_time) = json_ld_recipe.prep_time {
            if !prep_time.is_empty() {
                metadata.insert("prep time".to_string(), convert_duration(&prep_time));
            }
        }

        if let Some(cook_time) = json_ld_recipe.cook_time {
            if !cook_time.is_empty() {
                metadata.insert("cook time".to_string(), convert_duration(&cook_time));
            }
        }

        // Map cuisine
        if let Some(cuisine) = json_ld_recipe.recipe_cuisine {
            let cuisine_str = match cuisine {
                RecipeCuisine::String(s) => s,
                RecipeCuisine::Multiple(v) => v.join(", "),
            };
            if !cuisine_str.is_empty() {
                metadata.insert("cuisine".to_string(), cuisine_str);
            }
        }

        // Map diet restrictions
        if let Some(diet) = json_ld_recipe.suitable_for_diet {
            let diet_str = match diet {
                SuitableForDiet::String(s) => clean_diet_value(&s),
                SuitableForDiet::Multiple(v) => v
                    .iter()
                    .map(|d| clean_diet_value(d))
                    .collect::<Vec<String>>()
                    .join(", "),
            };
            metadata.insert("diet".to_string(), diet_str);
        }

        // Map keywords as tags
        if let Some(keywords) = json_ld_recipe.keywords {
            let tags = match keywords {
                Keywords::String(s) => s,
                Keywords::Multiple(v) => v.join(", "),
            };
            if !tags.is_empty() {
                metadata.insert("tags".to_string(), tags);
            }
        }

        // Map image (use the first image if multiple are available)
        if let Some(ref img) = json_ld_recipe.image {
            let image_url = match img {
                ImageType::String(i) => Some(decode_html_symbols(i)),
                ImageType::MultipleStrings(imgs) if !imgs.is_empty() => {
                    Some(decode_html_symbols(&imgs[0]))
                }
                ImageType::Object(i) => Some(i.url.clone()),
                ImageType::MultipleObjects(imgs) if !imgs.is_empty() => Some(imgs[0].url.clone()),
                _ => None,
            };
            if let Some(url) = image_url {
                if !url.is_empty() {
                    metadata.insert("image".to_string(), url);
                }
            }
        }

        // Combine ingredients and instructions into a single content field
        let ingredients = match json_ld_recipe.recipe_ingredient {
            Some(RecipeIngredients::Strings(ingredients)) => ingredients
                .into_iter()
                .filter(|ing| !ing.trim().is_empty())
                .map(|ing| decode_html_symbols(&ing))
                .collect::<Vec<String>>()
                .join("\n"),
            Some(RecipeIngredients::Objects(ingredients)) => ingredients
                .into_iter()
                .filter(|ing| !ing.name.trim().is_empty())
                .map(|ing| {
                    let amount = ing.amount.as_deref().unwrap_or("").trim();
                    let name = decode_html_symbols(&ing.name);
                    if amount.is_empty() {
                        name
                    } else {
                        format!("{amount} {name}")
                    }
                })
                .collect::<Vec<String>>()
                .join("\n"),
            None => String::new(),
        };

        let instructions = match json_ld_recipe.recipe_instructions {
            Some(instructions) => match instructions {
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
                            // Prefer text over name
                            if let Some(text) = step.text {
                                texts.push(text);
                            } else if let Some(name) = step.name {
                                texts.push(name);
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
                                // Prefer text over name
                                if let Some(text) = step.text {
                                    texts.push(text);
                                } else if let Some(name) = step.name {
                                    texts.push(name);
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
                RecipeInstructions::NestedSections(sections) => sections
                    .into_iter()
                    .flat_map(|section| {
                        section.into_iter().flat_map(|howto| match howto {
                            HowTo::HowToStep(step) => {
                                let mut texts = Vec::new();
                                if let Some(text) = step.text {
                                    texts.push(text);
                                } else if let Some(name) = step.name {
                                    texts.push(name);
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
                                    } else if let Some(name) = step.name {
                                        texts.push(name);
                                    }
                                    if let Some(desc) = step.description {
                                        texts.push(desc);
                                    }
                                    texts
                                })
                                .collect(),
                        })
                    })
                    .map(|text| decode_html_symbols(&text))
                    .collect::<Vec<String>>()
                    .join(" "),
            },
            None => String::new(),
        };

        // Combine into single content field
        let content = if !ingredients.is_empty() && !instructions.is_empty() {
            format!("{}\n\n{}", ingredients, instructions)
        } else if !ingredients.is_empty() {
            ingredients
        } else {
            instructions
        };

        Recipe {
            name: decode_html_symbols(&json_ld_recipe.name),
            description: json_ld_recipe.description.and_then(|desc| match desc {
                DescriptionType::String(d) => {
                    let decoded = decode_html_symbols(&d);
                    if decoded.is_empty() {
                        None
                    } else {
                        Some(decoded)
                    }
                }
                DescriptionType::Object(d) => {
                    let decoded = decode_html_symbols(&d.text);
                    if decoded.is_empty() {
                        None
                    } else {
                        Some(decoded)
                    }
                }
            }),
            image: json_ld_recipe.image.map_or(vec![], |img| match img {
                ImageType::String(i) => vec![decode_html_symbols(&i)],
                ImageType::MultipleStrings(imgs) => {
                    imgs.into_iter().map(|i| decode_html_symbols(&i)).collect()
                }
                ImageType::MultipleObjects(imgs) => imgs.into_iter().map(|i| i.url).collect(),
                ImageType::None => vec![],
                ImageType::Object(i) => vec![i.url],
            }),
            content,
            metadata,
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonLdRecipe {
    name: String,
    description: Option<DescriptionType>,
    image: Option<ImageType>,
    #[serde(rename = "recipeIngredient")]
    recipe_ingredient: Option<RecipeIngredients>,
    #[serde(rename = "recipeInstructions")]
    recipe_instructions: Option<RecipeInstructions>,
    #[serde(rename = "recipeYield")]
    recipe_yield: Option<RecipeYield>,
    #[serde(rename = "prepTime")]
    prep_time: Option<String>,
    #[serde(rename = "cookTime")]
    cook_time: Option<String>,
    #[serde(rename = "totalTime")]
    total_time: Option<String>,
    #[serde(rename = "suitableForDiet")]
    suitable_for_diet: Option<SuitableForDiet>,
    #[serde(rename = "recipeCategory")]
    recipe_category: Option<RecipeCategory>,
    #[serde(rename = "recipeCuisine")]
    recipe_cuisine: Option<RecipeCuisine>,
    keywords: Option<Keywords>,
    author: Option<Author>,
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
enum RecipeIngredients {
    Strings(Vec<String>),
    Objects(Vec<IngredientObject>),
}

#[derive(Debug, Deserialize)]
struct IngredientObject {
    name: String,
    amount: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RecipeInstructions {
    String(String),
    Multiple(Vec<String>),
    MultipleObject(Vec<RecipeInstructionObject>),
    HowTo(Vec<HowTo>),
    NestedSections(Vec<Vec<HowTo>>),
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
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "@type")]
struct HowToSection {
    #[serde(rename = "itemListElement")]
    item_list_element: Vec<HowToStep>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RecipeYield {
    String(String),
    Number(i32),
    Array(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SuitableForDiet {
    String(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Keywords {
    String(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Author {
    String(String),
    Object(AuthorObject),
    Multiple(Vec<AuthorObject>),
}

#[derive(Debug, Deserialize)]
struct AuthorObject {
    name: Option<String>,
    #[serde(rename = "@id")]
    _id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RecipeCategory {
    String(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RecipeCuisine {
    String(String),
    Multiple(Vec<String>),
}

impl TryFrom<&Value> for JsonLdRecipe {
    type Error = serde_json::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value.clone())
    }
}

fn decode_html_symbols(text: &str) -> String {
    // for some reason need to decode twice to get the correct string
    decode_html_entities(&decode_html_entities(text)).into_owned()
}

fn clean_diet_value(diet: &str) -> String {
    // Remove schema.org URLs and clean up diet values
    diet.trim_start_matches("https://schema.org/")
        .trim_start_matches("http://schema.org/")
        .replace("Diet", "")
        .trim()
        .to_string()
}

fn convert_duration(duration: &str) -> String {
    // Convert ISO 8601 duration to human-readable format
    // e.g., PT30M -> 30 minutes, PT1H30M -> 1 hour 30 minutes
    // Also handle ranges like PT15-20M and seconds like PT5400.0S
    if let Some(duration) = duration.strip_prefix("PT") {
        let mut result = String::new();

        // Handle hours
        if let Some(h_pos) = duration.find('H') {
            let hours: u32 = duration[..h_pos].parse().unwrap_or(0);
            result.push_str(&format!(
                "{} hour{}",
                hours,
                if hours == 1 { "" } else { "s" }
            ));
        }

        // Handle minutes (including ranges)
        if let Some(m_pos) = duration.find('M') {
            let start = duration.find('H').map(|p| p + 1).unwrap_or(0);
            let minutes_str = &duration[start..m_pos];

            // Check if it's a range (e.g., "15-20")
            if minutes_str.contains('-') {
                // For ranges, just use the full range string
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&format!("{minutes_str} minutes"));
            } else if let Ok(minutes) = minutes_str.parse::<u32>() {
                // Convert minutes > 60 to hours and minutes
                if minutes >= 60 {
                    let hours = minutes / 60;
                    let remaining_minutes = minutes % 60;

                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&format!(
                        "{} hour{}",
                        hours,
                        if hours == 1 { "" } else { "s" }
                    ));

                    if remaining_minutes > 0 {
                        result.push_str(&format!(
                            " {} minute{}",
                            remaining_minutes,
                            if remaining_minutes == 1 { "" } else { "s" }
                        ));
                    }
                } else {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&format!(
                        "{} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    ));
                }
            }
        }

        // Handle seconds (including decimal values like 5400.0S)
        if let Some(s_pos) = duration.find('S') {
            let start = duration.rfind(['H', 'M']).map(|p| p + 1).unwrap_or(0);
            let seconds_str = &duration[start..s_pos];

            if let Ok(seconds) = seconds_str.parse::<f64>() {
                let total_minutes = (seconds / 60.0).round() as u32;
                let hours = total_minutes / 60;
                let minutes = total_minutes % 60;

                result.clear(); // Clear any existing result

                if hours > 0 {
                    result.push_str(&format!(
                        "{} hour{}",
                        hours,
                        if hours == 1 { "" } else { "s" }
                    ));
                }

                if minutes > 0 {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&format!(
                        "{} minute{}",
                        minutes,
                        if minutes == 1 { "" } else { "s" }
                    ));
                }
            }
        }

        if result.is_empty() {
            duration.to_string()
        } else {
            result
        }
    } else {
        duration.to_string()
    }
}

fn is_recipe_type(value: &Value) -> bool {
    if let Some(type_value) = value.get("@type") {
        if let Some(type_str) = type_value.as_str() {
            return type_str.eq_ignore_ascii_case("recipe");
        }
    }
    false
}

impl Extractor for JsonLdExtractor {
    fn parse(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn std::error::Error>> {
        debug!("JsonLdExtractor: Starting parse for URL: {}", context.url);
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();
        let document = &context.document;

        let scripts: Vec<_> = document.select(&selector).collect();
        debug!(
            "JsonLdExtractor: Found {} JSON-LD script tags",
            scripts.len()
        );

        // Try each script element until we find a valid recipe
        for (index, script) in scripts.iter().enumerate() {
            let raw_json = script.inner_html();
            debug!(
                "JsonLdExtractor: Script {} raw content: {}",
                index, raw_json
            );

            let cleaned_json = sanitize_json(&raw_json);
            match serde_json::from_str::<Value>(&cleaned_json) {
                Ok(json_ld) => {
                    debug!(
                        "JsonLdExtractor: Successfully parsed JSON-LD {}: {:#?}",
                        index, json_ld
                    );

                    let recipe_json = if json_ld.is_array() {
                        debug!("JsonLdExtractor: JSON-LD is an array");
                        json_ld.as_array().and_then(|arr| {
                            arr.iter()
                                .find(|item| {
                                    let has_instructions = item.get("recipeInstructions").is_some();
                                    let is_recipe = is_recipe_type(item);
                                    debug!("JsonLdExtractor: Array item - has_instructions: {}, is_recipe: {}", has_instructions, is_recipe);
                                    has_instructions || is_recipe
                                })
                        })
                    } else if is_recipe_type(&json_ld) {
                        debug!("JsonLdExtractor: Found Recipe type in root");
                        Some(&json_ld)
                    } else if let Some(graph) = json_ld.get("@graph") {
                        debug!("JsonLdExtractor: Found @graph");
                        graph.as_array().and_then(|arr| {
                            arr.iter().find(|item| {
                                let is_recipe = is_recipe_type(item);
                                debug!("JsonLdExtractor: @graph item - is_recipe: {}", is_recipe);
                                is_recipe
                            })
                        })
                    } else {
                        debug!("JsonLdExtractor: No recipe found in this JSON-LD");
                        None
                    };

                    if let Some(recipe) = recipe_json {
                        debug!("JsonLdExtractor: Found recipe JSON: {:#?}", recipe);
                        match JsonLdRecipe::try_from(recipe) {
                            Ok(recipe) => {
                                debug!("JsonLdExtractor: Successfully converted to JsonLdRecipe");
                                return Ok(self.convert_to_recipe(recipe, &context.url));
                            }
                            Err(e) => {
                                debug!("JsonLdExtractor: Failed to convert to JsonLdRecipe: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("JsonLdExtractor: Failed to parse JSON-LD {}: {}", index, e);
                }
            }
        }

        let error_msg = "No valid recipe found in any JSON-LD script";
        debug!("JsonLdExtractor: {}", error_msg);
        Err(error_msg.into())
    }
}

fn sanitize_json(json_str: &str) -> String {
    debug!("Original JSON: {}", json_str);

    let mut minified = String::with_capacity(json_str.len());
    let mut in_string = false;
    let mut prev_char = None;
    let mut depth = 0;
    let chars: Vec<char> = json_str.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' if prev_char != Some('\\') => {
                in_string = !in_string;
                if !in_string {
                    // We're ending a string - check if we need a comma
                    let rest_chars = chars.get(i + 1..).unwrap_or(&[]);
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
                if let Some(rest_chars) = chars.get(i + 1..) {
                    let next_char = rest_chars.iter().find(|&&c| !c.is_whitespace());
                    if depth > 0 && matches!(next_char, Some(&'"')) {
                        debug!("Adding missing comma after array/object closing");
                        minified.push(',');
                        prev_char = Some(',');
                        continue;
                    }
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

    fn create_html_document(json_ld: &str) -> String {
        format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                    {json_ld}
                </script>
            </head>
            <body></body>
            </html>
            "#
        )
    }

    #[test]
    fn test_parse_success() {
        let html = "<html><body>Test</body></html>";
        let document = Html::parse_document(html);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };
        let extractor = JsonLdExtractor;
        // Just verify that parse returns an error for invalid input
        assert!(extractor.parse(&context).is_err());
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
            "recipeInstructions": "Mix ingredients. Bake at 350F for 10 minutes.",
            "author": "Jane Doe",
            "prepTime": "PT15M",
            "cookTime": "PT10M",
            "totalTime": "PT25M",
            "recipeYield": "24 cookies",
            "recipeCategory": "Dessert",
            "recipeCuisine": "American",
            "keywords": "chocolate, cookies, baking"
        }
        "#;
        let html_str = create_html_document(json_ld);
        let document = Html::parse_document(&html_str);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };

        let result = extractor.parse(&context).unwrap();

        assert_eq!(result.name, "Chocolate Chip Cookies");
        assert_eq!(
            result.description,
            Some("Delicious homemade cookies".to_string())
        );
        assert_eq!(result.image, vec!["https://example.com/cookie.jpg"]);
        assert_eq!(
            result.content,
            "flour\nsugar\nchocolate chips\n\nMix ingredients. Bake at 350F for 10 minutes."
        );

        // Test metadata mappings
        assert_eq!(result.metadata.get("source").unwrap(), "http://example.com");
        assert_eq!(result.metadata.get("author").unwrap(), "Jane Doe");
        assert_eq!(result.metadata.get("prep time").unwrap(), "15 minutes");
        assert_eq!(result.metadata.get("cook time").unwrap(), "10 minutes");
        assert_eq!(result.metadata.get("time required").unwrap(), "25 minutes");
        assert_eq!(result.metadata.get("servings").unwrap(), "24 cookies");
        assert_eq!(result.metadata.get("course").unwrap(), "Dessert");
        assert_eq!(result.metadata.get("cuisine").unwrap(), "American");
        assert_eq!(
            result.metadata.get("tags").unwrap(),
            "chocolate, cookies, baking"
        );
    }

    #[test]
    fn test_duration_conversion() {
        assert_eq!(convert_duration("PT30M"), "30 minutes");
        assert_eq!(convert_duration("PT1H"), "1 hour");
        assert_eq!(convert_duration("PT1H30M"), "1 hour 30 minutes");
        assert_eq!(convert_duration("PT90M"), "1 hour 30 minutes");
        assert_eq!(convert_duration("PT2H15M"), "2 hours 15 minutes");
        assert_eq!(convert_duration("invalid"), "invalid");
        // Test ranges
        assert_eq!(convert_duration("PT15-20M"), "15-20 minutes");
        assert_eq!(convert_duration("PT25-30M"), "25-30 minutes");
        // Test seconds
        assert_eq!(convert_duration("PT5400S"), "1 hour 30 minutes");
        assert_eq!(convert_duration("PT5400.0S"), "1 hour 30 minutes");
        assert_eq!(convert_duration("PT300S"), "5 minutes");
        // Test large minute values
        assert_eq!(convert_duration("PT150M"), "2 hours 30 minutes");
        assert_eq!(convert_duration("PT180M"), "3 hours");
        assert_eq!(convert_duration("PT65M"), "1 hour 5 minutes");
    }

    #[test]
    fn test_metadata_with_source_url() {
        let extractor = JsonLdExtractor;
        let json_ld = r#"
        {
            "@context": "https://schema.org/",
            "@type": "Recipe",
            "name": "Test Recipe",
            "description": "A test recipe",
            "image": "https://example.com/image.jpg",
            "recipeIngredient": ["ingredient 1"],
            "recipeInstructions": "Step 1",
            "suitableForDiet": "GlutenFree",
            "keywords": ["healthy", "quick", "easy"]
        }
        "#;
        let html_str = create_html_document(json_ld);
        let document = Html::parse_document(&html_str);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };

        let result = extractor.parse(&context).unwrap();

        assert_eq!(result.metadata.get("diet").unwrap(), "GlutenFree");
        assert_eq!(result.metadata.get("tags").unwrap(), "healthy, quick, easy");
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
                ],
                "author": {
                    "@type": "Person",
                    "name": "Chef Mario"
                },
                "recipeYield": 4,
                "suitableForDiet": ["GlutenFree", "LowCarb"],
                "recipeCuisine": "Italian"
            },
            {
                "@type": "WebSite",
                "name": "Recipe Website"
            }
        ]
        "#;
        let html_str = create_html_document(json_ld);
        let document = Html::parse_document(&html_str);
        let context = ParsingContext {
            url: "http://example.com".to_string(),
            document,
            texts: None,
        };

        let result = extractor.parse(&context).unwrap();

        assert_eq!(result.name, "Pasta Carbonara");
        assert_eq!(
            result.description,
            Some("Classic Italian pasta dish".to_string())
        );
        assert_eq!(
            result.image,
            vec![
                "https://example.com/carbonara1.jpg",
                "https://example.com/carbonara2.jpg"
            ]
        );
        assert_eq!(
            result.content,
            "spaghetti\neggs\nbacon\ncheese\n\nCook pasta Fry bacon Mix eggs and cheese Combine all ingredients"
        );

        // Test metadata extraction for complex types
        assert_eq!(result.metadata.get("author").unwrap(), "Chef Mario");
        assert_eq!(result.metadata.get("servings").unwrap(), "4");
        assert_eq!(result.metadata.get("diet").unwrap(), "GlutenFree, LowCarb");
        assert_eq!(result.metadata.get("cuisine").unwrap(), "Italian");
    }
}

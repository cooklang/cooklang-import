use crate::extractors::Extractor;
use crate::model::Recipe;
use html_escape::decode_html_entities;
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
}

impl TryFrom<Value> for JsonLdRecipe {
    type Error = serde_json::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value)
    }
}

fn decode_html_symbols(text: &str) -> String {
    decode_html_entities(&decode_html_entities(text)).into_owned()
}

impl From<JsonLdRecipe> for Recipe {
    fn from(json_ld_recipe: JsonLdRecipe) -> Self {
        Recipe {
            // for some reason need to decode twice to get the correct string
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
            },
        }
    }
}

impl Extractor for JsonLdExtractor {
    fn can_parse(&self, document: &Html) -> bool {
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();
        document.select(&selector).next().is_some()
    }

    fn parse(&self, document: &Html) -> Result<Recipe, Box<dyn std::error::Error>> {
        let selector = Selector::parse("script[type='application/ld+json']").unwrap();

        let script_content = document
            .select(&selector)
            .next()
            .ok_or("No JSON-LD script found")?
            .inner_html();

        let json_ld: serde_json::Value = serde_json::from_str(&script_content)?;
        println!("{:#?}", json_ld);

        let json_ld_recipe: JsonLdRecipe = if json_ld.is_array() {
            // If it's an array, find the first object of type "Recipe"
            json_ld
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item.get("recipeInstructions").is_some())
                })
                .ok_or("No Recipe object found in the JSON-LD array")?
                .clone()
                .try_into()?
        } else {
            // If it's a single object, use it directly
            json_ld.try_into()?
        };

        // Use the From trait to convert JsonLdRecipe to Recipe
        Ok(Recipe::from(json_ld_recipe))
    }
}

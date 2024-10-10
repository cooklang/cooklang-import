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
    HowToSection(Vec<HowToSection>),
    HowToSteps(Vec<HowToStep>),
}

#[derive(Debug, Deserialize)]
struct HowToStep {
    text: String,
}

#[derive(Debug, Deserialize)]
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
                RecipeInstructions::HowToSection(sections) => sections
                    .into_iter()
                    .flat_map(|section| section.item_list_element)
                    .map(|step| decode_html_symbols(&step.text))
                    .collect::<Vec<String>>()
                    .join(" "),
                RecipeInstructions::HowToSteps(steps) => steps
                    .into_iter()
                    .map(|step| decode_html_symbols(&step.text))
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

        let json_ld_recipe: JsonLdRecipe = if json_ld.is_array() {
            // If it's an array, find the first object of type "Recipe"
            let recipe = json_ld
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item.get("recipeInstructions").is_some())
                })
                .ok_or("No Recipe object found in the JSON-LD array")?
                .clone();

            debug!("{:#?}", recipe);

            recipe.try_into()?
        } else if let Some(graph) = json_ld.get("@graph") {
            // If it's an object with a "@graph" array, find the first object of type "Recipe"
            let recipe =graph
                .as_array()
                .and_then(|arr| {
                    arr.iter()
                        .find(|item| item.get("@type") == Some(&Value::String("Recipe".to_string())))
                })
                .ok_or("No Recipe object found in the @graph array")?
                .clone();

            debug!("{:#?}", recipe);

            recipe.try_into()?
        } else {
            debug!("{:#?}", json_ld);
            // If it's a single object, use it directly
            json_ld.try_into()?
        };

        // Use the From trait to convert JsonLdRecipe to Recipe
        Ok(Recipe::from(json_ld_recipe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;

    fn create_html_document(json_ld: &str) -> Html {
        let html = format!(
            r#"
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
        let extractor = JsonLdExtractor;
        let document = create_html_document("{}");
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

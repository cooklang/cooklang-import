pub mod converters;
pub mod extractors;
pub mod model;

use log::{debug, error};
use reqwest::header::{HeaderMap, USER_AGENT};
use scraper::Html;

use crate::converters::ConvertToCooklang;
use crate::extractors::Extractor;

pub fn import_recipe_raw(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Set up headers with a user agent
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse()?);

    // Fetch the webpage content with headers
    let body = reqwest::blocking::Client::new()
        .get(url)
        .headers(headers.clone())
        .send()?
        .text()?;

    // Parse the HTML document
    let document = Html::parse_document(&body);

    let extractor = extractors::JsonLdExtractor;
    if extractor.can_parse(&document) {
        let recipe = extractor.parse(&document)?;
        debug!("{:#?}", recipe);
        Ok(format!(
            "# {}\n\n## Ingredients\n{}\n\n## Steps\n{}",
            recipe.name,
            recipe.ingredients
                .iter()
                .map(|i| format!("- {}", i))
                .collect::<Vec<_>>()
                .join("\n"),
            recipe.steps
        ))
    } else {
        error!("No extractor found to parse the recipe from this webpage.");
        Err("No extractor found to parse the recipe from this webpage.".into())
    }
}

pub fn import_recipe(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Set up headers with a user agent
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse()?);

    // Fetch the webpage content with headers
    let body = reqwest::blocking::Client::new()
        .get(url)
        .headers(headers.clone())
        .send()?
        .text()?;

    // Parse the HTML document
    let document = Html::parse_document(&body);

    let extractor = extractors::JsonLdExtractor;
    if extractor.can_parse(&document) {
        let recipe = extractor.parse(&document)?;
        debug!("{:#?}", recipe);

        let openai_api_key =
            std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in the environment");
        let converter = converters::OpenAIConverter::new(openai_api_key);
        let cooklang_recipe = converter.convert(&recipe.ingredients, &recipe.steps)?;
        Ok(cooklang_recipe)
    } else {
        error!("Unable to parse the recipe from this webpage.");
        Err("Unable to parse the recipe from this webpage.".into())
    }
}

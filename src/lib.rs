pub mod converters;
pub mod extractors;
pub mod model;

use log::{debug, error};
use reqwest::blocking::get;
use scraper::Html;

use crate::converters::ConvertToCooklang;
use crate::extractors::Extractor;

pub fn import_recipe(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Fetch the webpage content
    let body = get(url)?.text()?;

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

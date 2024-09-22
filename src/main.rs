use log::{debug, error};
use reqwest::blocking::get;
use scraper::Html;
use std::env;

mod converters;
mod extractors;
mod model;

use crate::converters::ConvertToCooklang;
use crate::extractors::Extractor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the URL from command-line arguments
    let args: Vec<String> = env::args().collect();
    let url = args.get(1).ok_or("Please provide a URL as an argument")?;

    // Fetch the webpage content
    let body = get(url)?.text()?;

    // Parse the HTML document
    let document = Html::parse_document(&body);

    let extractor = extractors::JsonLdExtractor;
    if extractor.can_parse(&document) {
        let recipe = extractor.parse(&document)?;
        debug!("{:#?}", recipe);

        let openai_api_key =
            env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in the environment");
        let converter = converters::OpenAIConverter::new(openai_api_key);
        let cooklang_recipe = converter.convert(&recipe.ingredients, &recipe.steps)?;
        println!("{}", cooklang_recipe);
    } else {
        error!("Unable to parse the recipe from this webpage.");
    }

    Ok(())
}

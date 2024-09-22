use crate::extractors::Extractor;
use reqwest::blocking::get;
use scraper::Html;
use std::env;

mod extractors;
mod model;

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
        println!("{:#?}", recipe);
    } else {
        println!("Unable to parse the recipe from this webpage.");
    }

    Ok(())
}

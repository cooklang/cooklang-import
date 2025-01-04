pub mod converters;
pub mod extractors;
pub mod model;

use log::debug;
use reqwest::header::{HeaderMap, USER_AGENT};
use scraper::Html;

use crate::converters::ConvertToCooklang;
use crate::extractors::{Extractor, ParsingContext};

pub async fn fetch_recipe(url: &str) -> Result<model::Recipe, Box<dyn std::error::Error>> {
    // Set up headers with a user agent
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse()?);

    // Fetch the webpage content with headers
    let body = reqwest::Client::new()
        .get(url)
        .headers(headers.clone())
        .send()
        .await?
        .text()
        .await?;

    let context = ParsingContext {
        url: url.to_string(),
        document: Html::parse_document(&body),
        texts: None,
    };

    let extractors_list: Vec<Box<dyn Extractor>> = vec![
        Box::new(extractors::JsonLdExtractor),
        Box::new(extractors::PlainTextLlmExtractor),
    ];

    let mut last_error = None;
    for extractor in extractors_list {
        match extractor.parse(&context) {
            Ok(recipe) => {
                debug!("{:#?}", recipe);
                return Ok(recipe);
            }
            Err(e) => {
                debug!("Extractor failed: {}", e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| "No extractor could parse the recipe from this webpage.".into()))
}

pub async fn convert_recipe(recipe: &model::Recipe) -> Result<String, Box<dyn std::error::Error>> {
    let openai_api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in the environment");
    let model = std::env::var("OPENAI_MODEL").unwrap_or("gpt-4".to_string());

    let converter = converters::OpenAIConverter::new(openai_api_key, model);
    converter
        .convert(&recipe.ingredients, &recipe.instructions)
        .await
}

pub async fn import_recipe(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let recipe = fetch_recipe(url).await?;
    convert_recipe(&recipe).await
}

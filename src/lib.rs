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
        Box::new(extractors::HtmlClassExtractor),
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

pub fn generate_frontmatter(metadata: &std::collections::HashMap<String, String>) -> String {
    if metadata.is_empty() {
        return String::new();
    }

    let mut frontmatter = String::from("---\n");

    // Sort keys for consistent output
    let mut keys: Vec<_> = metadata.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = metadata.get(key) {
            // Escape values that contain special characters
            if value.contains('\n') || value.contains('"') || value.contains(':') {
                frontmatter.push_str(&format!("{}: \"{}\"\n", key, value.replace('"', "\\\"")));
            } else {
                frontmatter.push_str(&format!("{key}: {value}\n"));
            }
        }
    }

    frontmatter.push_str("---\n\n");
    frontmatter
}

pub async fn convert_recipe(recipe: &model::Recipe) -> Result<String, Box<dyn std::error::Error>> {
    let openai_api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in the environment");
    let model = std::env::var("OPENAI_MODEL").unwrap_or("gpt-4".to_string());

    let converter = converters::OpenAIConverter::new(openai_api_key, model);

    // Convert using the basic convert method
    let mut cooklang_recipe = converter
        .convert(&recipe.ingredients, &recipe.instructions)
        .await?;

    // Prepend frontmatter if there's metadata
    let frontmatter = generate_frontmatter(&recipe.metadata);
    if !frontmatter.is_empty() {
        cooklang_recipe = frontmatter + &cooklang_recipe;
    }

    Ok(cooklang_recipe)
}

pub async fn import_recipe(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let recipe = fetch_recipe(url).await?;
    convert_recipe(&recipe).await
}

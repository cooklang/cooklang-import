use super::RecipeComponents;
use crate::url_to_text::fetchers::{ChromeFetcher, RequestFetcher};
use crate::url_to_text::html::extractors::{
    Extractor, HtmlClassExtractor, JsonLdExtractor, MicroDataExtractor, ParsingContext,
};
use crate::url_to_text::text::TextExtractor;
use scraper::Html;
use std::error::Error;
use std::time::Duration;

/// Process a URL to extract recipe content
///
/// This pipeline:
/// 1. Fetches HTML using RequestFetcher
/// 2. Tries HTML extractors (json_ld, microdata, html_class) in order
/// 3. Falls back to ChromeFetcher (if available) or plain text extraction
/// 4. Uses TextExtractor for plain text extraction
/// 5. Returns RecipeComponents with separated text, metadata, and name
///
/// # Arguments
/// * `url` - The URL to fetch and process
///
/// # Returns
/// * `Ok(RecipeComponents)` - The extracted recipe components
/// * `Err(...)` - If all extraction methods fail
pub async fn process(url: &str) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    // 1. Fetch HTML
    let fetcher = RequestFetcher::new(Some(Duration::from_secs(30)));
    let html_content = fetcher.fetch(url).await?;
    let document = Html::parse_document(&html_content);

    let context = ParsingContext {
        url: url.to_string(),
        document,
        texts: None,
    };

    // 2. Try HTML extractors in order
    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(JsonLdExtractor),
        Box::new(MicroDataExtractor),
        Box::new(HtmlClassExtractor),
    ];

    for extractor in extractors {
        if let Ok(recipe) = extractor.parse(&context) {
            return Ok(recipe_to_components(&recipe));
        }
    }

    // 3. Fallback: plain text path
    let plain_text = if ChromeFetcher::is_available() {
        // Use ChromeFetcher if PAGE_SCRIBER_URL is configured
        let chrome =
            ChromeFetcher::new().ok_or("ChromeFetcher is available but failed to initialize")?;
        chrome.fetch(url).await?
    } else {
        // Extract text directly from HTML
        extract_text_from_html(&html_content)
    };

    // 4. Use TextExtractor to parse the plain text
    let text_with_metadata = TextExtractor::extract(&plain_text, url).await?;

    // Parse the text format and return as components
    Ok(parse_text_to_components(&text_with_metadata))
}

/// Convert a Recipe to RecipeComponents
fn recipe_to_components(recipe: &crate::model::Recipe) -> RecipeComponents {
    // Build text from ingredients and instructions
    let mut text = String::new();
    for ingredient in &recipe.ingredients {
        text.push_str(ingredient.trim());
        text.push('\n');
    }
    // Always add a blank line between ingredients and instructions
    if !recipe.ingredients.is_empty() && !recipe.instructions.is_empty() {
        text.push('\n');
    }
    text.push_str(recipe.instructions.trim_start());

    // Build metadata YAML (without --- delimiters)
    let mut metadata_lines = Vec::new();
    if let Some(desc) = &recipe.description {
        metadata_lines.push(format!("description: {}", desc));
    }
    // Only use the first image if multiple are available
    if let Some(first_image) = recipe.image.first() {
        metadata_lines.push(format!("image: {}", first_image));
    }
    for (key, value) in &recipe.metadata {
        metadata_lines.push(format!("{}: {}", key, value));
    }

    RecipeComponents {
        text,
        metadata: metadata_lines.join("\n"),
        name: recipe.name.clone(),
    }
}

/// Parse text format (with optional frontmatter) into RecipeComponents
fn parse_text_to_components(text: &str) -> RecipeComponents {
    let (metadata_map, body) = crate::model::Recipe::parse_text_format(text);

    // Extract name from metadata
    let name = metadata_map.get("title").cloned().unwrap_or_default();

    // Build metadata string excluding title (since it's in name field)
    let metadata_lines: Vec<String> = metadata_map
        .iter()
        .filter(|(k, _)| *k != "title")
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect();

    RecipeComponents {
        text: body,
        metadata: metadata_lines.join("\n"),
        name,
    }
}

/// Simple text extraction from HTML
///
/// Extracts all text content from the <body> element.
/// This is a basic fallback when structured extractors fail.
fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = scraper::Selector::parse("body").unwrap();
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"
            <html>
            <body>
                <h1>Test Recipe</h1>
                <p>Some ingredients</p>
                <p>Some instructions</p>
            </body>
            </html>
        "#;

        let text = extract_text_from_html(html);
        assert!(text.contains("Test Recipe"));
        assert!(text.contains("Some ingredients"));
        assert!(text.contains("Some instructions"));
    }
}

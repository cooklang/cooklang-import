use super::RecipeComponents;
use crate::url_to_text::text::TextExtractor;
use std::error::Error;

pub async fn process(
    text: &str,
    extract: bool,
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    if extract {
        // Run through LLM extractor
        let extracted = TextExtractor::extract(text, "direct-input").await?;
        Ok(parse_text_to_components(&extracted))
    } else {
        // Assume already formatted, parse it into components
        Ok(parse_text_to_components(text))
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

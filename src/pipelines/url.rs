use super::RecipeComponents;
use crate::config::load_config;
use crate::url_to_text::fetchers::{PageScriberFetcher, RequestFetcher};
use crate::url_to_text::html::extractors::{
    Extractor, HtmlClassExtractor, JsonLdExtractor, MicroDataExtractor, ParsingContext,
};
use crate::url_to_text::text::TextExtractor;
use scraper::Html;
use std::error::Error;
use std::time::Duration;

/// Process a URL to extract recipe content
///
/// Pipeline:
/// 1. Check if domain is in page_scriber.domains → use PageScriberFetcher
/// 2. Otherwise, use RequestFetcher
/// 3. Try structured extractors (JSON-LD → MicroData → HtmlClass)
/// 4. If RequestFetcher failed (402/blocked), auto-fallback to PageScriberFetcher
/// 5. Final fallback: TextExtractor (LLM) on extracted text
pub async fn process(url: &str) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let page_scriber_config = load_config()
        .ok()
        .map(|c| c.page_scriber)
        .unwrap_or_default();

    let use_page_scriber_first = domain_in_list(url, &page_scriber_config.domains);

    // Step 1: Fetch HTML — either via page scriber (for listed domains) or reqwest
    let (html_result, used_page_scriber) = if use_page_scriber_first {
        match PageScriberFetcher::new(page_scriber_config.url.clone()) {
            Some(fetcher) => (fetcher.fetch(url).await, true),
            None => {
                // Page scriber not configured despite domain being listed — fall back to reqwest
                let fetcher = RequestFetcher::new(Some(Duration::from_secs(30)));
                (fetcher.fetch(url).await, false)
            }
        }
    } else {
        let fetcher = RequestFetcher::new(Some(Duration::from_secs(30)));
        (fetcher.fetch(url).await, false)
    };

    // Step 2: If we got HTML, try structured extractors
    if let Ok(html_content) = &html_result {
        if let Some(components) = try_structured_extractors(html_content, url) {
            return Ok(components);
        }
    }

    // Step 3: If reqwest failed, auto-fallback to page scriber
    if !used_page_scriber && html_result.is_err() {
        if let Some(fetcher) = PageScriberFetcher::new(page_scriber_config.url.clone()) {
            if let Ok(html_content) = fetcher.fetch(url).await {
                if let Some(components) = try_structured_extractors(&html_content, url) {
                    return Ok(components);
                }
                // Structured extractors failed on page scriber HTML — try LLM
                if TextExtractor::is_available() {
                    let plain_text = extract_text_from_html(&html_content);
                    return TextExtractor::extract(&plain_text, url).await;
                }
            }
        }
    }

    // Step 4: Final fallback — LLM text extraction from whatever HTML we have
    let html_content = html_result?;

    if !TextExtractor::is_available() {
        return Err("No recipe found on page. Structured data extractors failed and LLM extraction is not configured.".into());
    }

    let plain_text = extract_text_from_html(&html_content);
    TextExtractor::extract(&plain_text, url).await
}

/// Try all structured extractors on HTML content.
/// Returns Some(RecipeComponents) if any extractor succeeds, None otherwise.
fn try_structured_extractors(html_content: &str, url: &str) -> Option<RecipeComponents> {
    let document = Html::parse_document(html_content);

    let context = ParsingContext {
        url: url.to_string(),
        document,
        texts: None,
    };

    let extractors: Vec<Box<dyn Extractor>> = vec![
        Box::new(JsonLdExtractor),
        Box::new(MicroDataExtractor),
        Box::new(HtmlClassExtractor),
    ];

    for extractor in extractors {
        if let Ok(recipe) = extractor.parse(&context) {
            return Some(recipe_to_components(&recipe));
        }
    }

    None
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
        metadata_lines.push(format!("description: {}", super::yaml_escape(desc)));
    }
    // Only use the first image if multiple are available
    if let Some(first_image) = recipe.image.first() {
        metadata_lines.push(format!("image: {}", super::yaml_escape(first_image)));
    }
    for (key, value) in &recipe.metadata {
        metadata_lines.push(format!("{}: {}", key, super::yaml_escape(value)));
    }

    RecipeComponents {
        text,
        metadata: metadata_lines.join("\n"),
        name: super::sanitize_name(&recipe.name),
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

/// Check if a URL's domain matches any domain in the list (suffix-matched).
/// "seriouseats.com" matches "www.seriouseats.com", "m.seriouseats.com", etc.
fn domain_in_list(url: &str, domains: &[String]) -> bool {
    let host = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("");

    domains
        .iter()
        .any(|domain| host == domain.as_str() || host.ends_with(&format!(".{}", domain)))
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

    #[test]
    fn test_domain_matches_exact() {
        let domains = vec!["seriouseats.com".to_string()];
        assert!(domain_in_list("https://seriouseats.com/recipe", &domains));
    }

    #[test]
    fn test_domain_matches_subdomain() {
        let domains = vec!["seriouseats.com".to_string()];
        assert!(domain_in_list(
            "https://www.seriouseats.com/recipe",
            &domains
        ));
    }

    #[test]
    fn test_domain_no_match() {
        let domains = vec!["seriouseats.com".to_string()];
        assert!(!domain_in_list("https://example.com/recipe", &domains));
    }

    #[test]
    fn test_domain_empty_list() {
        let domains: Vec<String> = vec![];
        assert!(!domain_in_list("https://seriouseats.com/recipe", &domains));
    }

    #[test]
    fn test_domain_invalid_url() {
        let domains = vec!["seriouseats.com".to_string()];
        assert!(!domain_in_list("not-a-url", &domains));
    }
}

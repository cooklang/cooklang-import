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
/// 5. Returns the Recipe's to_text_with_metadata() output
///
/// # Arguments
/// * `url` - The URL to fetch and process
///
/// # Returns
/// * `Ok(String)` - The extracted recipe in text format with metadata
/// * `Err(...)` - If all extraction methods fail
pub async fn process(url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
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
            return Ok(recipe.to_text_with_metadata());
        }
    }

    // 3. Fallback: plain text path
    let plain_text = if ChromeFetcher::is_available() {
        // Use ChromeFetcher if PAGE_SCRIBER_URL is configured
        let chrome = ChromeFetcher::new()
            .ok_or("ChromeFetcher is available but failed to initialize")?;
        chrome.fetch(url).await?
    } else {
        // Extract text directly from HTML
        extract_text_from_html(&html_content)
    };

    // 4. Use TextExtractor to parse the plain text
    let text_with_metadata = TextExtractor::extract(&plain_text, url).await?;

    // For now, return the text as-is (conversion will be added later)
    Ok(text_with_metadata)
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

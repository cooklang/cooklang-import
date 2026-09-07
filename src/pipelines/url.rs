use super::RecipeComponents;
use crate::url_to_text::fetchers::{DynFetcher, Fetcher, PageScriberFetcher, RequestFetcher};
use crate::url_to_text::html::extractors::{
    Extractor, HtmlClassExtractor, JsonLdExtractor, MicroDataExtractor, ParsingContext,
};
use crate::url_to_text::text::TextExtractor;
use log::debug;
use scraper::Html;
use std::error::Error;
use std::time::Duration;

/// Process a URL to extract recipe content
///
/// Pipeline:
/// 1. Check if domain is in page_scriber.domains → use PageScriberFetcher
/// 2. Otherwise, use RequestFetcher
/// 3. Try structured extractors (JSON-LD → MicroData → HtmlClass)
/// 4. If RequestFetcher failed (402/blocked), or returned a bot-challenge page under
///    a 200, auto-fallback to PageScriberFetcher
/// 5. Final fallback: TextExtractor (LLM) on extracted text
///
/// The extracted title is normalized afterwards (SEO padding stripped,
/// generated when the page provides none).
pub async fn process(url: &str) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let fetchers: Vec<DynFetcher> = vec![
        Box::new(RequestFetcher::new(Some(Duration::from_secs(30)))),
        Box::new(PageScriberFetcher::from_env()),
    ];
    let mut components = fetch_and_extract_with(url, fetchers).await?;
    super::title::ensure_title(&mut components).await;
    Ok(components)
}

pub async fn process_with(
    url: &str,
    fetchers: Vec<DynFetcher>,
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let mut components = fetch_and_extract_with(url, fetchers).await?;

    super::title::ensure_title(&mut components).await;
    Ok(components)
}

fn order_fetchers(url: &str, fetchers: Vec<DynFetcher>) -> Vec<Box<dyn Fetcher>> {
    // filters out any unavailable fetchers, and any fetcher which is not configured for the url
    // and should not be used as fallback, then stable sorts the list based on the fetchers being
    // configured for the specific url (putting matches first).
    let mut new_fetchers = fetchers
        .into_iter()
        .filter(|f| {
            if !f.is_available() {
                return false;
            }

            // If not specifically configured for this url and the fetcher should not
            // be used as a fallback method
            if !f.is_configured(url) & !f.fallback() {
                return false;
            }

            true
        })
        .collect::<Vec<_>>();

    new_fetchers.sort_by_key(|b| std::cmp::Reverse(b.is_configured(url)));

    new_fetchers
}

async fn try_fetch_and_extract(
    url: &str,
    fetcher: &DynFetcher,
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let html = fetcher.fetch(url).await?;
    if let Some(components) = try_structured_extractors(&html, url) {
        return Ok(components);
    }

    // Some sites answer a plain HTTP client with 200 and a bot-challenge or
    // JavaScript-shell page. That is a failed fetch in every sense that matters, so
    // demote it to an error.
    //
    // This runs *after* the structured extractors above, deliberately: a page whose
    // body is rendered client-side still ships usable JSON-LD in <head>, and that is
    // a successful extraction, not a blocked fetch.
    if looks_blocked(&html) {
        return Err("Blocked or empty page returned by direct fetch".into());
    }

    if !TextExtractor::is_available() {
        return Err("No recipe found on page. Structured data extractors failed and LLM extraction is not configured.".into());
    }

    TextExtractor::extract(&extract_text_from_html(&html), url).await
}

// Try all fetchers starting with fetchers configured for the url (otherwise stable ordering),
async fn fetch_and_extract_with(
    url: &str,
    fetchers: Vec<Box<dyn Fetcher>>,
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    let mut iter = order_fetchers(url, fetchers).into_iter().peekable();

    while let Some(fetcher) = iter.next() {
        let last = iter.peek().is_none();
        match try_fetch_and_extract(url, &fetcher).await {
            Ok(components) => return Ok(components),
            Err(e) => {
                if last {
                    // No more fetchers to try, just return the latest error
                    return Err(e);
                }
                debug!(
                    "Fetching {url} with {} failed: {e}. Retrying...",
                    fetcher.name()
                );
            }
        }
    }

    unreachable!()
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
            let components = recipe_to_components(&recipe);
            // A structured hit with no ingredients and no instructions is worse than
            // no hit at all: it stops the pipeline before the remaining extractors and
            // the LLM fallback run, and the caller ends up converting an empty recipe.
            if components.text.trim().is_empty() {
                continue;
            }
            return Some(components);
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
    let mut entries = Vec::new();
    if let Some(desc) = &recipe.description {
        entries.push(("description".to_string(), desc.clone()));
    }
    // Only use the first image if multiple are available
    if let Some(first_image) = recipe.image.first() {
        entries.push(("image".to_string(), first_image.clone()));
    }
    for (key, value) in &recipe.metadata {
        if key == "servings" {
            // Reduce free-form yields ("Makes 12", "4 personnes") to a numeric
            // servings value; the original text is kept under `yield`.
            entries.extend(super::servings_entries(value));
        } else {
            entries.push((key.clone(), value.clone()));
        }
    }

    RecipeComponents {
        text,
        metadata: super::metadata_to_yaml(&entries),
        name: super::sanitize_name(&recipe.name),
    }
}

/// Upper bound on the plain text handed to the LLM extractor.
///
/// gpt-4o-mini's window is 128k tokens. Minified JavaScript and CSS tokenize far
/// worse than prose (often under 2 chars/token), so the cap is set well below a
/// naive 4-chars-per-token estimate. Recipes live near the top of a page, so
/// truncating the tail loses far less than overflowing the request loses.
pub(crate) const MAX_LLM_INPUT_CHARS: usize = 120_000;

/// Text nodes that are markup machinery rather than page content.
///
/// scraper's `.text()` walks every descendant text node, and html5ever stores a
/// <script> body as a text node child of the script element - so without this the
/// LLM receives the page's JavaScript. On the pages sampled from the 2026-08-17..24
/// failures this was 69-99% of the payload.
const NON_CONTENT_SELECTOR: &str = "script, style, noscript, template, svg, iframe";

/// Simple text extraction from HTML
///
/// Extracts the human-readable text of the <body> element, skipping script/style
/// machinery, collapsing whitespace and capping the length.
/// This is a basic fallback when structured extractors fail.
fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let body = scraper::Selector::parse("body").unwrap();
    let non_content = scraper::Selector::parse(NON_CONTENT_SELECTOR).unwrap();

    let Some(body_el) = document.select(&body).next() else {
        return String::new();
    };

    // Node ids of every element whose text is machinery, so their text nodes can be
    // skipped during the walk.
    let excluded: std::collections::HashSet<_> =
        body_el.select(&non_content).map(|el| el.id()).collect();

    let mut out = String::new();
    for node in body_el.descendants() {
        let Some(text) = node.value().as_text() else {
            continue;
        };
        // Skip text belonging to an excluded element (at any depth).
        if node.ancestors().any(|a| excluded.contains(&a.id())) {
            continue;
        }
        for word in text.split_whitespace() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(word);
            if out.len() >= MAX_LLM_INPUT_CHARS {
                out.truncate(MAX_LLM_INPUT_CHARS);
                return out;
            }
        }
    }
    out
}

/// Heuristic for a page that returned HTTP 200 but is a bot challenge, consent
/// interstitial or an empty JavaScript shell rather than the requested content.
///
/// kingarthurbaking.com is the motivating case: it answers plain reqwest with 200
/// and a 3 KB "Client Challenge" page. Treating that as a successful fetch meant the
/// page-scriber fallback never fired and the LLM was asked to find a recipe in a
/// JavaScript shim.
pub(crate) fn looks_blocked(html: &str) -> bool {
    const BLOCK_TITLES: [&str; 10] = [
        "client challenge",
        "just a moment",
        "attention required",
        "access denied",
        "access to this page has been denied",
        "are you a robot",
        "verify you are a human",
        "security check",
        "bot verification",
        "please enable javascript",
    ];

    let document = Html::parse_document(html);

    if let Some(title) = scraper::Selector::parse("title")
        .ok()
        .and_then(|sel| document.select(&sel).next())
    {
        let title = title.text().collect::<String>().to_lowercase();
        if BLOCK_TITLES.iter().any(|t| title.contains(t)) {
            return true;
        }
    }

    // An almost-empty body from a 200 response is a JS shell, not a recipe page.
    // Measured on the 2026-08-17..24 failures: real recipe pages carried 3,918-38,979
    // chars of visible text, while challenge/shell pages carried 0-211. 500 sits in
    // that gap with an order of magnitude of headroom on the content side.
    extract_text_from_html(html).len() < 500
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::PageScriberConfig;
    use crate::url_to_text::fetchers::domain_in_list;

    fn to_string_vec<S: AsRef<str>>(arr: &[S]) -> Vec<String> {
        arr.iter()
            .map(|s: &S| s.as_ref().to_string())
            .collect::<Vec<String>>()
    }

    #[track_caller]
    fn assert_fetcher_order(url: &str, fetchers: Vec<DynFetcher>, expected: &[&str]) {
        let ordered = order_fetchers(url, fetchers);

        let names = ordered
            .iter()
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>();

        assert_eq!(names, to_string_vec(expected));
    }

    struct MockFetcher {
        name: String,
        available: bool,
        fallback: bool,
        domains: Vec<String>,
    }

    #[async_trait::async_trait]
    impl Fetcher for MockFetcher {
        async fn fetch(
            &self,
            _url: &str,
        ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            unimplemented!()
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn is_configured(&self, url: &str) -> bool {
            domain_in_list(url, &self.domains)
        }

        fn fallback(&self) -> bool {
            self.fallback
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    fn mock_fetcher_no_fallback(name: &str, domains: &[&str]) -> DynFetcher {
        Box::new(MockFetcher {
            name: name.to_string(),
            domains: to_string_vec(domains),
            fallback: false,
            available: true,
        })
    }

    #[test]
    fn test_order_fetchers() {
        let new_page_scriber = |domains: &[&str]| {
            PageScriberFetcher::new(PageScriberConfig {
                url: Some("http://localhost:4000".to_string()),
                domains: to_string_vec(domains),
            })
        };

        // Pagescriber is not available -> should only try `request`
        assert_fetcher_order(
            "https://www.seriouseats.com/recipe",
            vec![
                Box::new(RequestFetcher::default()),
                Box::new(PageScriberFetcher::new(PageScriberConfig::default())),
            ],
            &["request"],
        );

        // Pagescriber is available but not configured for the url -> keep same order
        assert_fetcher_order(
            "https://www.seriouseats.com/recipe",
            vec![
                Box::new(RequestFetcher::default()),
                Box::new(new_page_scriber(&[])),
            ],
            &["request", "page_scriber"],
        );

        // Pagescriber is available and configured for the url -> order page_scriber first
        assert_fetcher_order(
            "https://www.seriouseats.com/recipe",
            vec![
                Box::new(RequestFetcher::default()),
                Box::new(new_page_scriber(&["seriouseats.com"])),
            ],
            &["page_scriber", "request"],
        );

        // Fetcher without fallback that does not match
        assert_fetcher_order(
            "https://www.seriouseats.com/recipe",
            vec![
                mock_fetcher_no_fallback("special", &["example.com"]),
                Box::new(RequestFetcher::default()),
                Box::new(new_page_scriber(&["seriouseats.com"])),
            ],
            &["page_scriber", "request"],
        );

        // Fetcher with fallback that matches
        assert_fetcher_order(
            "https://www.seriouseats.com/recipe",
            vec![
                mock_fetcher_no_fallback("special", &["seriouseats.com"]),
                mock_fetcher_no_fallback("special-2", &["seriouseats.com"]),
                Box::new(RequestFetcher::default()),
                Box::new(new_page_scriber(&["seriouseats.com"])),
            ],
            &["special", "special-2", "page_scriber", "request"],
        );
    }

    // --- Regression: LLM fallback input hygiene (production failures 2026-08-17..24) ---
    //
    // scraper's `.text()` yields the text nodes of <script> and <style> too, so the
    // plain text handed to the LLM was 69-99% minified JavaScript on every failing
    // page sampled. That both drowned the recipe ("No recipe found in the text") and
    // blew the model's context window (one page reached 924,697 tokens).

    #[test]
    fn test_extract_text_from_html_drops_script_and_style() {
        let html = r#"<html><body>
            <h1>Tomato Soup</h1>
            <script>var tracker={"id":1};function noise(){return "not a recipe"}</script>
            <style>.hero{color:red}</style>
            <noscript>Please enable JavaScript</noscript>
            <template><div>hidden clone</div></template>
            <p>500 g tomatoes</p>
        </body></html>"#;
        let text = extract_text_from_html(html);
        assert!(text.contains("Tomato Soup"));
        assert!(text.contains("500 g tomatoes"));
        assert!(!text.contains("tracker"), "script text leaked: {text}");
        assert!(!text.contains("noise"), "script text leaked: {text}");
        assert!(!text.contains("color:red"), "style text leaked: {text}");
        assert!(
            !text.contains("enable JavaScript"),
            "noscript leaked: {text}"
        );
        assert!(!text.contains("hidden clone"), "template leaked: {text}");
    }

    #[test]
    fn test_extract_text_from_html_collapses_whitespace() {
        let html = "<html><body><p>a</p>\n\n\n     <p>b</p>          <p>c</p></body></html>";
        let text = extract_text_from_html(html);
        assert_eq!(text, "a b c");
    }

    #[test]
    fn test_extract_text_from_html_is_capped() {
        // A 3 MB page must not be sent to the model verbatim.
        let filler = "word ".repeat(400_000);
        let html = format!("<html><body><p>{filler}</p></body></html>");
        let text = extract_text_from_html(&html);
        assert!(
            text.len() <= MAX_LLM_INPUT_CHARS,
            "expected <= {} chars, got {}",
            MAX_LLM_INPUT_CHARS,
            text.len()
        );
    }

    // --- Regression: bot-challenge pages served with HTTP 200 ---
    //
    // kingarthurbaking.com answers reqwest with 200 and a 3 KB "Client Challenge"
    // interstitial. Because the fetch "succeeded", the page-scriber fallback never
    // fired and the LLM was asked to find a recipe in a JavaScript shim.

    #[test]
    fn test_detects_client_challenge_page() {
        let html = r#"<!DOCTYPE html><html lang="en"><head><title>Client Challenge</title>
            </head><body><noscript>JavaScript is disabled in your browser.</noscript>
            <script>loadScript('/challenge.js')</script></body></html>"#;
        assert!(looks_blocked(html));
    }

    #[test]
    fn test_detects_other_block_pages() {
        for title in [
            "Just a moment...",
            "Attention Required! | Cloudflare",
            "Access denied",
            "Are you a robot?",
            "Please verify you are a human",
        ] {
            let html = format!("<html><head><title>{title}</title></head><body></body></html>");
            assert!(
                looks_blocked(&html),
                "should flag block page titled {title:?}"
            );
        }
    }

    #[test]
    fn test_real_page_is_not_flagged_as_blocked() {
        // Body text sized like a real recipe page (the smallest sampled in production
        // carried 3,918 chars of visible text).
        let steps = "Preheat the oven to 425F and bake for 15 minutes. ".repeat(80);
        let html = format!(
            r#"<html><head><title>Buttermilk Biscuits Recipe</title></head>
            <body><h1>Buttermilk Biscuits</h1><p>2 cups flour</p><p>{steps}</p></body></html>"#
        );
        assert!(!looks_blocked(&html));
    }

    #[test]
    fn test_tiny_body_is_flagged_as_blocked() {
        // A near-empty body from a 200 response is a JS shell, not a recipe page.
        let html = "<html><head><title>Recipe</title></head><body><div id=\"app\"></div>                    <script>boot()</script></body></html>";
        assert!(looks_blocked(html));
    }

    // --- Regression: structured extractors returning empty content ---

    #[test]
    fn test_structured_extractors_reject_empty_result() {
        // A Recipe block with nothing but a name must not satisfy the pipeline.
        let html = r#"<html><head><script type="application/ld+json">
            {"@context":"https://schema.org","@type":"Recipe","name":"Ghost Recipe"}
            </script></head><body><p>nothing here</p></body></html>"#;
        assert!(
            try_structured_extractors(html, "http://example.com").is_none(),
            "empty structured result must fall through to the LLM path"
        );
    }

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

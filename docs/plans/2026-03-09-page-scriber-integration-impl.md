# Page Scriber Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace ChromeFetcher with a new PageScriberFetcher that returns HTML source (not plain text), enabling structured extractors to work on browser-rendered content from bot-blocked sites.

**Architecture:** New `PageScriberFetcher` hits `/api/fetch-source` for HTML. Config gains `page_scriber` section with `url` and `domains` list. URL pipeline routes by domain: listed domains go straight to page scriber, others try reqwest first with auto-fallback on failure.

**Tech Stack:** Rust, reqwest, serde, config crate (TOML)

---

### Task 1: Add PageScriberConfig to config

**Files:**
- Modify: `src/config.rs:5-25` (add PageScriberConfig struct and field to AiConfig)

**Step 1: Write the failing test**

Add to `src/config.rs` tests module:

```rust
#[test]
fn test_page_scriber_config_default() {
    let config = PageScriberConfig::default();
    assert!(config.url.is_none());
    assert!(config.domains.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_page_scriber_config_default -- --nocapture`
Expected: FAIL — `PageScriberConfig` not found

**Step 3: Write minimal implementation**

Add to `src/config.rs`:

```rust
/// Configuration for the page scriber service (browser-based fetching)
#[derive(Debug, Deserialize, Clone, Default)]
pub struct PageScriberConfig {
    /// Base URL of the page scriber service (e.g., "http://localhost:4000")
    pub url: Option<String>,
    /// Domains that should use page scriber directly (suffix-matched)
    /// e.g., ["seriouseats.com", "allrecipes.com"]
    #[serde(default)]
    pub domains: Vec<String>,
}
```

Add field to `AiConfig`:

```rust
/// Page scriber configuration for browser-based fetching
#[serde(default)]
pub page_scriber: PageScriberConfig,
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_page_scriber_config_default -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add PageScriberConfig to config"
```

---

### Task 2: Create PageScriberFetcher

**Files:**
- Create: `src/url_to_text/fetchers/page_scriber.rs`
- Modify: `src/url_to_text/fetchers/mod.rs`

**Step 1: Write the failing test**

Add to `src/url_to_text/fetchers/page_scriber.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_none_without_url() {
        let fetcher = PageScriberFetcher::new(None);
        assert!(fetcher.is_none());
    }

    #[test]
    fn test_new_returns_some_with_url() {
        let fetcher = PageScriberFetcher::new(Some("http://localhost:4000".to_string()));
        assert!(fetcher.is_some());
    }

    #[test]
    fn test_is_available_without_url() {
        assert!(!PageScriberFetcher::is_available(None));
    }

    #[test]
    fn test_is_available_with_url() {
        assert!(PageScriberFetcher::is_available(Some(&"http://localhost:4000".to_string())));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test page_scriber -- --nocapture`
Expected: FAIL — module not found

**Step 3: Write minimal implementation**

Create `src/url_to_text/fetchers/page_scriber.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct SourceRequest {
    url: String,
}

#[derive(Deserialize)]
struct SourceResponse {
    content: String,
}

pub struct PageScriberFetcher {
    endpoint: String,
    client: Client,
}

impl PageScriberFetcher {
    pub fn new(page_scriber_url: Option<String>) -> Option<Self> {
        let base_url = page_scriber_url?;
        let endpoint = format!("{}/api/fetch-source", base_url);
        let client = Client::new();
        Some(Self { endpoint, client })
    }

    pub fn is_available(page_scriber_url: Option<&String>) -> bool {
        page_scriber_url.is_some()
    }

    /// Fetch HTML source from a URL via the page scriber service.
    /// Returns raw HTML that can be parsed by structured extractors.
    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(&SourceRequest {
                url: url.to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!(
                "Page scriber fetch failed with status: {}",
                response.status()
            )
            .into());
        }

        let content: SourceResponse = response.json().await?;
        Ok(content.content)
    }
}
```

Update `src/url_to_text/fetchers/mod.rs`:

```rust
mod page_scriber;
mod request;

pub use page_scriber::PageScriberFetcher;
pub use request::RequestFetcher;
```

**Step 4: Run test to verify it passes**

Run: `cargo test page_scriber -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/url_to_text/fetchers/page_scriber.rs src/url_to_text/fetchers/mod.rs
git commit -m "feat: add PageScriberFetcher for /api/fetch-source"
```

---

### Task 3: Add domain matching helper

**Files:**
- Modify: `src/pipelines/url.rs`

**Step 1: Write the failing test**

Add to `src/pipelines/url.rs` tests module:

```rust
#[test]
fn test_domain_matches_exact() {
    let domains = vec!["seriouseats.com".to_string()];
    assert!(domain_in_list("https://seriouseats.com/recipe", &domains));
}

#[test]
fn test_domain_matches_subdomain() {
    let domains = vec!["seriouseats.com".to_string()];
    assert!(domain_in_list("https://www.seriouseats.com/recipe", &domains));
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
```

**Step 2: Run test to verify they fail**

Run: `cargo test domain_ -- --nocapture`
Expected: FAIL — `domain_in_list` not found

**Step 3: Write minimal implementation**

Add to `src/pipelines/url.rs`:

```rust
/// Check if a URL's domain matches any domain in the list (suffix-matched).
/// "seriouseats.com" matches "www.seriouseats.com", "m.seriouseats.com", etc.
fn domain_in_list(url: &str, domains: &[String]) -> bool {
    let host = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("");

    domains.iter().any(|domain| {
        host == domain.as_str() || host.ends_with(&format!(".{}", domain))
    })
}
```

**Step 4: Run test to verify they pass**

Run: `cargo test domain_ -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/pipelines/url.rs
git commit -m "feat: add domain_in_list helper for page scriber routing"
```

---

### Task 4: Rewrite URL pipeline with domain-aware routing

**Files:**
- Modify: `src/pipelines/url.rs:1-75` (rewrite `process` function)

**Step 1: Write the failing test**

Add integration test `tests/test_page_scriber_routing.rs`:

```rust
use cooklang_import::url_to_recipe;

/// This test verifies that the pipeline can fetch from a site that blocks bots
/// when page scriber is configured. Requires:
/// 1. Page scriber running at http://localhost:4000
/// 2. Config with page_scriber.url and page_scriber.domains set
#[tokio::test]
#[ignore] // Requires page scriber running locally
async fn test_seriouseats_via_page_scriber() {
    let url = "https://www.seriouseats.com/macau-pork-chop-sandwich-recipe-8605269";

    match url_to_recipe(url).await {
        Ok(result) => {
            println!("Name: {}", result.name);
            assert!(!result.name.is_empty(), "Recipe name should not be empty");
            assert!(!result.text.is_empty(), "Recipe text should not be empty");
        }
        Err(e) => {
            panic!("Failed to fetch recipe via page scriber: {}", e);
        }
    }
}
```

**Step 2: Rewrite the `process` function**

Replace the entire `process` function in `src/pipelines/url.rs`:

```rust
use crate::config::load_config;

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
```

Update imports at top of `src/pipelines/url.rs`:

```rust
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
```

**Step 3: Run existing tests to verify nothing is broken**

Run: `cargo test -- --nocapture`
Expected: All existing tests PASS

**Step 4: Commit**

```bash
git add src/pipelines/url.rs tests/test_page_scriber_routing.rs
git commit -m "feat: rewrite URL pipeline with domain-aware page scriber routing"
```

---

### Task 5: Remove ChromeFetcher

**Files:**
- Delete: `src/url_to_text/fetchers/chrome.rs`
- Modify: `src/url_to_text/fetchers/mod.rs` (remove chrome module and re-export)

**Step 1: Verify no remaining references to ChromeFetcher**

Run: `grep -r "ChromeFetcher\|chrome::" src/ --include="*.rs"`
Expected: Only `src/url_to_text/fetchers/mod.rs` and `src/url_to_text/fetchers/chrome.rs`

**Step 2: Remove the file and update mod.rs**

Delete `src/url_to_text/fetchers/chrome.rs`.

Update `src/url_to_text/fetchers/mod.rs` to remove the chrome module (should already be done from Task 2, but verify):

```rust
mod page_scriber;
mod request;

pub use page_scriber::PageScriberFetcher;
pub use request::RequestFetcher;
```

**Step 3: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All tests PASS, no compilation errors

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: remove ChromeFetcher, replaced by PageScriberFetcher"
```

---

### Task 6: Verify end-to-end with page scriber

**Requires:** Page scriber running at `http://localhost:4000`

**Step 1: Create a test config file**

Create `config.toml` in the project root (if not already present) with:

```toml
[page_scriber]
url = "http://localhost:4000"
domains = ["seriouseats.com"]
```

**Step 2: Run the ignored integration test**

Run: `cargo test test_seriouseats_via_page_scriber -- --ignored --nocapture`
Expected: PASS — recipe extracted from Serious Eats via page scriber

**Step 3: Manual CLI test**

Run: `cargo run -- "https://www.seriouseats.com/macau-pork-chop-sandwich-recipe-8605269" --extract-only`
Expected: Recipe components printed with name, ingredients, instructions

**Step 4: Test auto-fallback (non-listed domain that blocks bots)**

If you know another site that returns 402, test it without adding it to domains list to verify auto-fallback works.

**Step 5: Commit config if needed**

If `config.toml` is project-level and should be committed, add it. Otherwise ensure it's in `.gitignore`.

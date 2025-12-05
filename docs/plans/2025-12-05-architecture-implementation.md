# Architecture Restructuring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure cooklang-import into clear modules: url_to_text, images_to_text, converters, and pipelines.

**Architecture:** Move from flat structure to hierarchical modules organized by input type. Fetchers shared between html/text paths. Pipelines orchestrate each flow. Builder handles routing and fallback logic.

**Tech Stack:** Rust, tokio, reqwest, scraper, serde, async-trait

---

## Phase 1: Create Module Structure (No Logic Changes)

### Task 1.1: Create directory structure

**Files:**
- Create: `src/url_to_text/mod.rs`
- Create: `src/url_to_text/fetchers/mod.rs`
- Create: `src/url_to_text/html/mod.rs`
- Create: `src/url_to_text/html/extractors/mod.rs`
- Create: `src/url_to_text/text/mod.rs`
- Create: `src/images_to_text/mod.rs`
- Create: `src/converters/mod.rs`
- Create: `src/pipelines/mod.rs`

**Step 1: Create all directories and empty mod.rs files**

```bash
mkdir -p src/url_to_text/fetchers
mkdir -p src/url_to_text/html/extractors
mkdir -p src/url_to_text/text
mkdir -p src/images_to_text
mkdir -p src/converters
mkdir -p src/pipelines
```

**Step 2: Create placeholder mod.rs files**

`src/url_to_text/mod.rs`:
```rust
pub mod fetchers;
pub mod html;
pub mod text;
```

`src/url_to_text/fetchers/mod.rs`:
```rust
// Fetchers: request.rs, chrome.rs
```

`src/url_to_text/html/mod.rs`:
```rust
pub mod extractors;
```

`src/url_to_text/html/extractors/mod.rs`:
```rust
// Extractors: json_ld.rs, microdata.rs, html_class.rs
```

`src/url_to_text/text/mod.rs`:
```rust
// Plain text extraction: extractor.rs
```

`src/images_to_text/mod.rs`:
```rust
// OCR: ocr.rs
```

`src/converters/mod.rs`:
```rust
// Converters: open_ai.rs, anthropic.rs, etc.
```

`src/pipelines/mod.rs`:
```rust
// Pipelines: url.rs, text.rs, image.rs
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Success (empty modules are valid)

**Step 4: Commit**

```bash
git add src/url_to_text src/images_to_text src/converters src/pipelines
git commit -m "chore: create new module directory structure"
```

---

## Phase 2: Move Fetchers

### Task 2.1: Create request fetcher

**Files:**
- Create: `src/url_to_text/fetchers/request.rs`
- Modify: `src/url_to_text/fetchers/mod.rs`

**Step 1: Write the request fetcher module**

`src/url_to_text/fetchers/request.rs`:
```rust
use reqwest::Client;
use std::error::Error;
use std::time::Duration;

pub struct RequestFetcher {
    client: Client,
    timeout: Duration,
}

impl RequestFetcher {
    pub fn new(timeout: Option<Duration>) -> Self {
        let timeout = timeout.unwrap_or(Duration::from_secs(30));
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("Mozilla/5.0 (compatible; CooklangBot/1.0)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client, timeout }
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self.client.get(url).send().await?;
        let html = response.text().await?;
        Ok(html)
    }
}
```

**Step 2: Update mod.rs**

`src/url_to_text/fetchers/mod.rs`:
```rust
mod request;

pub use request::RequestFetcher;
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 4: Commit**

```bash
git add src/url_to_text/fetchers/
git commit -m "feat: add RequestFetcher to url_to_text/fetchers"
```

---

### Task 2.2: Create chrome fetcher

**Files:**
- Create: `src/url_to_text/fetchers/chrome.rs`
- Modify: `src/url_to_text/fetchers/mod.rs`

**Step 1: Write the chrome fetcher module**

Extract PAGE_SCRIBER_URL logic from `src/extractors/plain_text_llm.rs` (lines 93-118).

`src/url_to_text/fetchers/chrome.rs`:
```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

#[derive(Serialize)]
struct ContentRequest {
    url: String,
}

#[derive(Deserialize)]
struct ContentResponse {
    content: String,
}

pub struct ChromeFetcher {
    endpoint: String,
    client: Client,
}

impl ChromeFetcher {
    pub fn new() -> Option<Self> {
        let page_scriber_url = env::var("PAGE_SCRIBER_URL").ok()?;
        let endpoint = format!("{}/api/fetch-content", page_scriber_url);
        let client = Client::new();
        Some(Self { endpoint, client })
    }

    pub fn is_available() -> bool {
        env::var("PAGE_SCRIBER_URL").is_ok()
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self.client
            .post(&self.endpoint)
            .json(&ContentRequest { url: url.to_string() })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!(
                "Chrome fetch failed with status: {}",
                response.status()
            ).into());
        }

        let content: ContentResponse = response.json().await?;
        Ok(content.content)
    }
}
```

**Step 2: Update mod.rs**

`src/url_to_text/fetchers/mod.rs`:
```rust
mod request;
mod chrome;

pub use request::RequestFetcher;
pub use chrome::ChromeFetcher;
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 4: Commit**

```bash
git add src/url_to_text/fetchers/
git commit -m "feat: add ChromeFetcher to url_to_text/fetchers"
```

---

## Phase 3: Move HTML Extractors

### Task 3.1: Move json_ld extractor

**Files:**
- Move: `src/extractors/json_ld.rs` → `src/url_to_text/html/extractors/json_ld.rs`
- Modify: `src/url_to_text/html/extractors/mod.rs`

**Step 1: Copy json_ld.rs to new location**

```bash
cp src/extractors/json_ld.rs src/url_to_text/html/extractors/json_ld.rs
```

**Step 2: Update imports in new file if needed**

Review `src/url_to_text/html/extractors/json_ld.rs` and update any relative imports:
- Change `use crate::extractors::{Extractor, ParsingContext};` to define locally or re-export
- Change `use crate::model::Recipe;` stays the same

**Step 3: Update mod.rs**

`src/url_to_text/html/extractors/mod.rs`:
```rust
mod json_ld;

pub use json_ld::JsonLdExtractor;

// Re-export common types
use crate::model::Recipe;
use scraper::Html;
use std::error::Error;

pub struct ParsingContext {
    pub url: String,
    pub document: Html,
    pub texts: Option<String>,
}

pub trait Extractor: Send + Sync {
    fn extract(&self, context: &ParsingContext) -> Result<Recipe, Box<dyn Error + Send + Sync>>;
}
```

**Step 4: Verify project compiles**

Run: `cargo check`
Expected: May have errors - fix import issues

**Step 5: Commit**

```bash
git add src/url_to_text/html/extractors/
git commit -m "feat: move JsonLdExtractor to url_to_text/html/extractors"
```

---

### Task 3.2: Move microdata extractor

**Files:**
- Move: `src/extractors/microdata.rs` → `src/url_to_text/html/extractors/microdata.rs`
- Modify: `src/url_to_text/html/extractors/mod.rs`

**Step 1: Copy microdata.rs to new location**

```bash
cp src/extractors/microdata.rs src/url_to_text/html/extractors/microdata.rs
```

**Step 2: Update imports in new file**

Update `use crate::extractors::{Extractor, ParsingContext};` to `use super::{Extractor, ParsingContext};`

**Step 3: Update mod.rs**

`src/url_to_text/html/extractors/mod.rs`:
```rust
mod json_ld;
mod microdata;

pub use json_ld::JsonLdExtractor;
pub use microdata::MicroDataExtractor;

// ... rest stays the same
```

**Step 4: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 5: Commit**

```bash
git add src/url_to_text/html/extractors/
git commit -m "feat: move MicroDataExtractor to url_to_text/html/extractors"
```

---

### Task 3.3: Move html_class extractor

**Files:**
- Move: `src/extractors/html_class.rs` → `src/url_to_text/html/extractors/html_class.rs`
- Modify: `src/url_to_text/html/extractors/mod.rs`

**Step 1: Copy html_class.rs to new location**

```bash
cp src/extractors/html_class.rs src/url_to_text/html/extractors/html_class.rs
```

**Step 2: Update imports in new file**

Update `use crate::extractors::{Extractor, ParsingContext};` to `use super::{Extractor, ParsingContext};`

**Step 3: Update mod.rs**

`src/url_to_text/html/extractors/mod.rs`:
```rust
mod json_ld;
mod microdata;
mod html_class;

pub use json_ld::JsonLdExtractor;
pub use microdata::MicroDataExtractor;
pub use html_class::HtmlClassExtractor;

// ... rest stays the same
```

**Step 4: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 5: Commit**

```bash
git add src/url_to_text/html/extractors/
git commit -m "feat: move HtmlClassExtractor to url_to_text/html/extractors"
```

---

## Phase 4: Move Plain Text Extractor

### Task 4.1: Create text extractor

**Files:**
- Create: `src/url_to_text/text/extractor.rs`
- Modify: `src/url_to_text/text/mod.rs`

**Step 1: Extract LLM extraction logic from plain_text_llm.rs**

Copy the LLM extraction parts (not fetching) from `src/extractors/plain_text_llm.rs`.

`src/url_to_text/text/extractor.rs`:
```rust
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::error::Error;

const PROMPT: &str = r#"
You're an expert in finding recipe ingredients and instructions from messy texts.
Sometimes the text is not a recipe, in that case specify that in error field.
Given the text output only this JSON without any other characters:

{
  "ingredients": [<LIST OF INGREDIENTS HERE>],
  "instructions": [<LIST OF INSTRUCTIONS HERE>],
  "error": "<ERROR MESSAGE HERE IF NO RECIPE>"
}
"#;

const MODEL: &str = "gpt-4o-mini";

pub struct TextExtractor;

impl TextExtractor {
    pub async fn extract(
        plain_text: &str,
        source: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let json = fetch_json(plain_text.to_string()).await?;

        if let Some(error) = json["error"].as_str() {
            if !error.is_empty() {
                return Err(error.into());
            }
        }

        let ingredients = json["ingredients"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join("\n");

        let instructions = json["instructions"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join(" ");

        // Format as text with minimal frontmatter
        let mut output = String::new();
        output.push_str("---\n");
        output.push_str(&format!("source: {}\n", source));
        output.push_str("---\n\n");
        output.push_str(&ingredients);
        output.push_str("\n\n");
        output.push_str(&instructions);

        Ok(output)
    }
}

async fn fetch_json(texts: String) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let api_key = env::var("OPENAI_API_KEY")?;

    if api_key == "test_key" {
        return Ok(serde_json::json!({
            "ingredients": ["pasta", "sauce"],
            "instructions": ["Cook pasta with sauce"],
            "error": ""
        }));
    }

    let response = Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "messages": [
                { "role": "system", "content": PROMPT },
                { "role": "user", "content": texts }
            ]
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to get response content")?;

    serde_json::from_str(content).map_err(|e| e.into())
}
```

**Step 2: Update mod.rs**

`src/url_to_text/text/mod.rs`:
```rust
mod extractor;

pub use extractor::TextExtractor;
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 4: Commit**

```bash
git add src/url_to_text/text/
git commit -m "feat: add TextExtractor to url_to_text/text"
```

---

## Phase 5: Move OCR

### Task 5.1: Move OCR to images_to_text

**Files:**
- Move: `src/ocr.rs` → `src/images_to_text/ocr.rs`
- Modify: `src/images_to_text/mod.rs`

**Step 1: Copy ocr.rs to new location**

```bash
cp src/ocr.rs src/images_to_text/ocr.rs
```

**Step 2: Add ImageSource enum and update functions**

`src/images_to_text/ocr.rs` (add at top):
```rust
pub enum ImageSource {
    Path(String),
    Base64(String),
}

pub async fn extract(source: &ImageSource) -> Result<String, Box<dyn Error + Send + Sync>> {
    match source {
        ImageSource::Path(path) => extract_from_file(path).await,
        ImageSource::Base64(data) => extract_from_base64(data).await,
    }
}

async fn extract_from_file(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let data = tokio::fs::read(path).await?;
    let base64 = base64::encode(&data);
    call_google_vision(&base64).await
}

async fn extract_from_base64(data: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    call_google_vision(data).await
}

// Keep existing call_google_vision implementation
```

**Step 3: Update mod.rs**

`src/images_to_text/mod.rs`:
```rust
mod ocr;

pub use ocr::{ImageSource, extract};
```

**Step 4: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 5: Commit**

```bash
git add src/images_to_text/
git commit -m "feat: move OCR to images_to_text module"
```

---

## Phase 6: Move Converters (Providers)

### Task 6.1: Move converter trait and factory

**Files:**
- Create: `src/converters/mod.rs` (with trait)
- Move prompt.rs

**Step 1: Create converter trait**

`src/converters/mod.rs`:
```rust
mod prompt;
mod open_ai;
mod anthropic;
mod azure_openai;
mod google;
mod ollama;

pub use prompt::COOKLANG_CONVERTER_PROMPT;
pub use open_ai::OpenAiConverter;
pub use anthropic::AnthropicConverter;
pub use azure_openai::AzureOpenAiConverter;
pub use google::GoogleConverter;
pub use ollama::OllamaConverter;

use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Converter: Send + Sync {
    fn name(&self) -> &str;
    async fn convert(&self, ingredients_and_instructions: &str) -> Result<String, Box<dyn Error + Send + Sync>>;
}

pub fn create_converter(
    name: &str,
    config: &crate::config::ProviderConfig,
) -> Option<Box<dyn Converter>> {
    match name {
        "open_ai" => Some(Box::new(OpenAiConverter::new(config))),
        "anthropic" => Some(Box::new(AnthropicConverter::new(config))),
        "azure_openai" => Some(Box::new(AzureOpenAiConverter::new(config))),
        "google" => Some(Box::new(GoogleConverter::new(config))),
        "ollama" => Some(Box::new(OllamaConverter::new(config))),
        _ => None,
    }
}
```

**Step 2: Copy prompt.rs**

```bash
cp src/providers/prompt.rs src/converters/prompt.rs
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Errors (missing converter files)

**Step 4: Commit partial progress**

```bash
git add src/converters/mod.rs src/converters/prompt.rs
git commit -m "feat: add Converter trait and prompt to converters module"
```

---

### Task 6.2: Move OpenAI converter

**Files:**
- Move: `src/providers/open_ai.rs` → `src/converters/open_ai.rs`

**Step 1: Copy and adapt open_ai.rs**

```bash
cp src/providers/open_ai.rs src/converters/open_ai.rs
```

**Step 2: Update to implement Converter trait instead of LlmProvider**

Change `impl LlmProvider for OpenAiProvider` to `impl Converter for OpenAiConverter`
Rename struct from `OpenAiProvider` to `OpenAiConverter`

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: May have errors - continue with other converters

**Step 4: Commit**

```bash
git add src/converters/open_ai.rs
git commit -m "feat: move OpenAI to converters module"
```

---

### Task 6.3: Move Anthropic converter

**Files:**
- Move: `src/providers/anthropic.rs` → `src/converters/anthropic.rs`

**Step 1: Copy and adapt anthropic.rs**

```bash
cp src/providers/anthropic.rs src/converters/anthropic.rs
```

**Step 2: Update to implement Converter trait**

Rename struct from `AnthropicProvider` to `AnthropicConverter`

**Step 3: Commit**

```bash
git add src/converters/anthropic.rs
git commit -m "feat: move Anthropic to converters module"
```

---

### Task 6.4: Move remaining converters

**Files:**
- Move: `src/providers/azure_openai.rs` → `src/converters/azure_openai.rs`
- Move: `src/providers/google.rs` → `src/converters/google.rs`
- Move: `src/providers/ollama.rs` → `src/converters/ollama.rs`

**Step 1: Copy all remaining providers**

```bash
cp src/providers/azure_openai.rs src/converters/azure_openai.rs
cp src/providers/google.rs src/converters/google.rs
cp src/providers/ollama.rs src/converters/ollama.rs
```

**Step 2: Update each to implement Converter trait**

Rename structs to use `Converter` suffix.

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 4: Commit**

```bash
git add src/converters/
git commit -m "feat: move all providers to converters module"
```

---

## Phase 7: Update Recipe Model

### Task 7.1: Update Recipe struct

**Files:**
- Modify: `src/model.rs`

**Step 1: Update Recipe struct**

`src/model.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    pub image: Vec<String>,
    pub ingredients: Vec<String>,
    pub instructions: String,
    pub metadata: HashMap<String, String>,
}

impl Recipe {
    pub fn to_text_with_metadata(&self) -> String {
        let mut output = String::new();

        // Build metadata including name
        let mut metadata = self.metadata.clone();
        if !self.name.is_empty() {
            metadata.insert("title".to_string(), self.name.clone());
        }
        if let Some(desc) = &self.description {
            metadata.insert("description".to_string(), desc.clone());
        }

        // YAML frontmatter
        if !metadata.is_empty() {
            output.push_str("---\n");
            for (key, value) in &metadata {
                output.push_str(&format!("{}: {}\n", key, value));
            }
            output.push_str("---\n\n");
        }

        // Ingredients (one per line)
        for ingredient in &self.ingredients {
            output.push_str(ingredient);
            output.push('\n');
        }

        // Blank line separator
        output.push('\n');

        // Instructions
        output.push_str(&self.instructions);

        output
    }

    /// Extract frontmatter and body from text format
    pub fn parse_text_format(text: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();
        let body;

        if text.starts_with("---\n") {
            if let Some(end) = text[4..].find("\n---\n") {
                let frontmatter = &text[4..4 + end];
                for line in frontmatter.lines() {
                    if let Some((key, value)) = line.split_once(": ") {
                        metadata.insert(key.to_string(), value.to_string());
                    }
                }
                body = text[4 + end + 5..].to_string();
            } else {
                body = text.to_string();
            }
        } else {
            body = text.to_string();
        }

        (metadata, body)
    }
}
```

**Step 2: Verify project compiles**

Run: `cargo check`
Expected: Errors in extractors (they use old `content` field)

**Step 3: Commit model changes**

```bash
git add src/model.rs
git commit -m "feat: update Recipe with ingredients/instructions and serialization"
```

---

### Task 7.2: Update extractors to use new Recipe fields

**Files:**
- Modify: `src/url_to_text/html/extractors/json_ld.rs`
- Modify: `src/url_to_text/html/extractors/microdata.rs`
- Modify: `src/url_to_text/html/extractors/html_class.rs`

**Step 1: Update json_ld.rs**

Change Recipe construction from:
```rust
Recipe {
    name: ...,
    content: format!("{}\n\n{}", ingredients, instructions),
    ...
}
```

To:
```rust
Recipe {
    name: ...,
    ingredients: ingredients_vec,
    instructions: instructions_string,
    ...
}
```

**Step 2: Update microdata.rs similarly**

**Step 3: Update html_class.rs similarly**

**Step 4: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 5: Commit**

```bash
git add src/url_to_text/html/extractors/
git commit -m "fix: update extractors to use new Recipe fields"
```

---

## Phase 8: Create Pipelines

### Task 8.1: Create URL pipeline

**Files:**
- Create: `src/pipelines/url.rs`
- Modify: `src/pipelines/mod.rs`

**Step 1: Write URL pipeline**

`src/pipelines/url.rs`:
```rust
use crate::config::Config;
use crate::converters::{self, Converter};
use crate::model::Recipe;
use crate::url_to_text::fetchers::{ChromeFetcher, RequestFetcher};
use crate::url_to_text::html::extractors::{
    Extractor, HtmlClassExtractor, JsonLdExtractor, MicroDataExtractor, ParsingContext,
};
use crate::url_to_text::text::TextExtractor;
use scraper::Html;
use std::error::Error;

pub async fn process(url: &str, config: &Config) -> Result<String, Box<dyn Error + Send + Sync>> {
    // 1. Fetch HTML
    let fetcher = RequestFetcher::new(config.timeout);
    let html_content = fetcher.fetch(url).await?;
    let document = Html::parse_document(&html_content);

    let context = ParsingContext {
        url: url.to_string(),
        document,
        texts: None,
    };

    // 2. Try HTML extractors in configured order
    let extractors: Vec<Box<dyn Extractor>> = config
        .extractors
        .order
        .iter()
        .filter(|name| config.extractors.enabled.contains(name))
        .filter_map(|name| create_extractor(name))
        .collect();

    for extractor in extractors {
        if let Ok(recipe) = extractor.extract(&context) {
            let text = recipe.to_text_with_metadata();
            return convert_to_cooklang(&text, config).await;
        }
    }

    // 3. Fallback: plain text path
    let plain_text = if ChromeFetcher::is_available() {
        let chrome = ChromeFetcher::new().unwrap();
        chrome.fetch(url).await?
    } else {
        extract_text_from_html(&html_content)
    };

    let text = TextExtractor::extract(&plain_text, url).await?;
    convert_to_cooklang(&text, config).await
}

fn create_extractor(name: &str) -> Option<Box<dyn Extractor>> {
    match name {
        "json_ld" => Some(Box::new(JsonLdExtractor)),
        "microdata" => Some(Box::new(MicroDataExtractor)),
        "html_class" => Some(Box::new(HtmlClassExtractor)),
        _ => None,
    }
}

fn extract_text_from_html(html: &str) -> String {
    // Simple text extraction from HTML
    let document = Html::parse_document(html);
    let selector = scraper::Selector::parse("body").unwrap();
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

async fn convert_to_cooklang(
    text: &str,
    config: &Config,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let (metadata, body) = Recipe::parse_text_format(text);

    // Try converters in order
    for name in &config.converters.order {
        if !config.converters.enabled.contains(name) {
            continue;
        }
        if let Some(converter) = converters::create_converter(name, &config.get_converter_config(name)) {
            match converter.convert(&body).await {
                Ok(cooklang_body) => {
                    return Ok(assemble_output(&metadata, &cooklang_body));
                }
                Err(_) => continue,
            }
        }
    }

    Err("All converters failed".into())
}

fn assemble_output(metadata: &std::collections::HashMap<String, String>, cooklang_body: &str) -> String {
    let mut output = String::new();

    if !metadata.is_empty() {
        output.push_str("---\n");
        for (key, value) in metadata {
            output.push_str(&format!("{}: {}\n", key, value));
        }
        output.push_str("---\n\n");
    }

    output.push_str(cooklang_body);
    output
}
```

**Step 2: Update mod.rs**

`src/pipelines/mod.rs`:
```rust
pub mod url;
pub mod text;
pub mod image;
```

**Step 3: Verify project compiles**

Run: `cargo check`
Expected: Errors (missing text/image pipelines)

**Step 4: Commit**

```bash
git add src/pipelines/
git commit -m "feat: add URL pipeline"
```

---

### Task 8.2: Create text pipeline

**Files:**
- Create: `src/pipelines/text.rs`

**Step 1: Write text pipeline**

`src/pipelines/text.rs`:
```rust
use crate::config::Config;
use crate::converters;
use crate::model::Recipe;
use crate::url_to_text::text::TextExtractor;
use std::error::Error;

pub async fn process(
    text: &str,
    config: &Config,
    extract: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let formatted_text = if extract {
        // Run through LLM extractor
        TextExtractor::extract(text, "direct-input").await?
    } else {
        // Assume already formatted
        text.to_string()
    };

    convert_to_cooklang(&formatted_text, config).await
}

async fn convert_to_cooklang(
    text: &str,
    config: &Config,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let (metadata, body) = Recipe::parse_text_format(text);

    for name in &config.converters.order {
        if !config.converters.enabled.contains(name) {
            continue;
        }
        if let Some(converter) = converters::create_converter(name, &config.get_converter_config(name)) {
            match converter.convert(&body).await {
                Ok(cooklang_body) => {
                    return Ok(assemble_output(&metadata, &cooklang_body));
                }
                Err(_) => continue,
            }
        }
    }

    Err("All converters failed".into())
}

fn assemble_output(metadata: &std::collections::HashMap<String, String>, cooklang_body: &str) -> String {
    let mut output = String::new();

    if !metadata.is_empty() {
        output.push_str("---\n");
        for (key, value) in metadata {
            output.push_str(&format!("{}: {}\n", key, value));
        }
        output.push_str("---\n\n");
    }

    output.push_str(cooklang_body);
    output
}
```

**Step 2: Verify project compiles**

Run: `cargo check`

**Step 3: Commit**

```bash
git add src/pipelines/text.rs
git commit -m "feat: add text pipeline"
```

---

### Task 8.3: Create image pipeline

**Files:**
- Create: `src/pipelines/image.rs`

**Step 1: Write image pipeline**

`src/pipelines/image.rs`:
```rust
use crate::config::Config;
use crate::converters;
use crate::images_to_text::{self, ImageSource};
use crate::model::Recipe;
use std::error::Error;

pub async fn process(
    images: &[ImageSource],
    config: &Config,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut all_text = Vec::new();
    let mut sources = Vec::new();

    for image in images {
        let text = images_to_text::extract(image).await?;
        all_text.push(text);

        match image {
            ImageSource::Path(p) => sources.push(p.clone()),
            ImageSource::Base64(_) => sources.push("base64-image".to_string()),
        }
    }

    let combined = all_text.join("\n\n");
    let source = sources.join(", ");

    // Format as text with frontmatter
    let formatted = format!("---\nsource: {}\n---\n\n{}", source, combined);

    convert_to_cooklang(&formatted, config).await
}

async fn convert_to_cooklang(
    text: &str,
    config: &Config,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let (metadata, body) = Recipe::parse_text_format(text);

    for name in &config.converters.order {
        if !config.converters.enabled.contains(name) {
            continue;
        }
        if let Some(converter) = converters::create_converter(name, &config.get_converter_config(name)) {
            match converter.convert(&body).await {
                Ok(cooklang_body) => {
                    return Ok(assemble_output(&metadata, &cooklang_body));
                }
                Err(_) => continue,
            }
        }
    }

    Err("All converters failed".into())
}

fn assemble_output(metadata: &std::collections::HashMap<String, String>, cooklang_body: &str) -> String {
    let mut output = String::new();

    if !metadata.is_empty() {
        output.push_str("---\n");
        for (key, value) in metadata {
            output.push_str(&format!("{}: {}\n", key, value));
        }
        output.push_str("---\n\n");
    }

    output.push_str(cooklang_body);
    output
}
```

**Step 2: Verify project compiles**

Run: `cargo check`

**Step 3: Commit**

```bash
git add src/pipelines/image.rs
git commit -m "feat: add image pipeline"
```

---

## Phase 9: Update Config

### Task 9.1: Add extractor/converter config

**Files:**
- Modify: `src/config.rs`

**Step 1: Add new config structs**

Add to `src/config.rs`:
```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractorsConfig {
    #[serde(default = "default_extractors")]
    pub enabled: Vec<String>,
    #[serde(default = "default_extractors")]
    pub order: Vec<String>,
}

fn default_extractors() -> Vec<String> {
    vec!["json_ld".to_string(), "microdata".to_string(), "html_class".to_string()]
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConvertersConfig {
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub order: Vec<String>,
    #[serde(default)]
    pub default: String,
}
```

**Step 2: Add to main Config struct**

```rust
pub struct Config {
    // ... existing fields
    pub extractors: ExtractorsConfig,
    pub converters: ConvertersConfig,
    pub timeout: Option<std::time::Duration>,
}
```

**Step 3: Verify project compiles**

Run: `cargo check`

**Step 4: Commit**

```bash
git add src/config.rs
git commit -m "feat: add extractors and converters config"
```

---

## Phase 10: Update Builder

### Task 10.1: Update InputSource enum

**Files:**
- Modify: `src/builder.rs`

**Step 1: Update InputSource**

```rust
use crate::images_to_text::ImageSource;

pub enum InputSource {
    Url(String),
    Text { content: String, extract: bool },
    Images(Vec<ImageSource>),
}
```

**Step 2: Update builder methods**

```rust
impl RecipeImporterBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.source = Some(InputSource::Url(url.into()));
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.source = Some(InputSource::Text {
            content: text.into(),
            extract: false
        });
        self
    }

    pub fn text_with_extraction(mut self, text: impl Into<String>) -> Self {
        self.source = Some(InputSource::Text {
            content: text.into(),
            extract: true
        });
        self
    }

    pub fn image_path(mut self, path: impl Into<String>) -> Self {
        match &mut self.source {
            Some(InputSource::Images(images)) => {
                images.push(ImageSource::Path(path.into()));
            }
            _ => {
                self.source = Some(InputSource::Images(vec![ImageSource::Path(path.into())]));
            }
        }
        self
    }

    pub fn image_base64(mut self, data: impl Into<String>) -> Self {
        match &mut self.source {
            Some(InputSource::Images(images)) => {
                images.push(ImageSource::Base64(data.into()));
            }
            _ => {
                self.source = Some(InputSource::Images(vec![ImageSource::Base64(data.into())]));
            }
        }
        self
    }
}
```

**Step 3: Update build() to use pipelines**

```rust
pub async fn build(self) -> Result<String, crate::error::Error> {
    let config = self.build_config()?;

    match self.source.ok_or(crate::error::Error::NoInputSource)? {
        InputSource::Url(url) => {
            crate::pipelines::url::process(&url, &config)
                .await
                .map_err(|e| crate::error::Error::PipelineError(e.to_string()))
        }
        InputSource::Text { content, extract } => {
            crate::pipelines::text::process(&content, &config, extract)
                .await
                .map_err(|e| crate::error::Error::PipelineError(e.to_string()))
        }
        InputSource::Images(images) => {
            crate::pipelines::image::process(&images, &config)
                .await
                .map_err(|e| crate::error::Error::PipelineError(e.to_string()))
        }
    }
}
```

**Step 4: Verify project compiles**

Run: `cargo check`

**Step 5: Commit**

```bash
git add src/builder.rs
git commit -m "feat: update builder to use new pipelines"
```

---

## Phase 11: Update lib.rs and Clean Up

### Task 11.1: Update lib.rs exports

**Files:**
- Modify: `src/lib.rs`

**Step 1: Update module declarations and exports**

```rust
pub mod builder;
pub mod config;
pub mod converters;
pub mod error;
pub mod images_to_text;
pub mod model;
pub mod pipelines;
pub mod url_to_text;

// Re-exports for convenience
pub use builder::{RecipeImporter, RecipeImporterBuilder};
pub use config::Config;
pub use error::Error;
pub use images_to_text::ImageSource;
pub use model::Recipe;

// Convenience functions (simplified)
pub async fn import_from_url(url: &str) -> Result<String, Error> {
    RecipeImporter::builder().url(url).build().await
}

pub async fn convert_text_to_cooklang(text: &str) -> Result<String, Error> {
    RecipeImporter::builder().text(text).build().await
}
```

**Step 2: Verify project compiles**

Run: `cargo check`

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: update lib.rs exports for new architecture"
```

---

### Task 11.2: Remove old modules

**Files:**
- Delete: `src/extractors/` (old directory)
- Delete: `src/providers/` (old directory)
- Delete: `src/ocr.rs` (old file)

**Step 1: Remove old directories**

```bash
rm -rf src/extractors
rm -rf src/providers
rm -f src/ocr.rs
```

**Step 2: Verify project compiles**

Run: `cargo check`
Expected: Success

**Step 3: Run tests**

Run: `cargo test`
Expected: Tests may need updating

**Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove old module structure"
```

---

## Phase 12: Update Tests

### Task 12.1: Update integration tests

**Files:**
- Modify: `tests/*.rs`

**Step 1: Update imports in test files**

Change:
```rust
use cooklang_import::extractors::*;
use cooklang_import::providers::*;
```

To:
```rust
use cooklang_import::url_to_text::html::extractors::*;
use cooklang_import::converters::*;
```

**Step 2: Run tests**

Run: `cargo test`

**Step 3: Fix any failing tests**

**Step 4: Commit**

```bash
git add tests/
git commit -m "test: update tests for new architecture"
```

---

## Phase 13: Final Verification

### Task 13.1: Full build and test

**Step 1: Clean build**

```bash
cargo clean
cargo build
```

**Step 2: Run all tests**

```bash
cargo test
```

**Step 3: Run clippy**

```bash
cargo clippy
```

**Step 4: Fix any warnings**

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: architecture restructuring complete"
```

---

## Summary

This plan restructures the codebase from:

```
src/
├── extractors/
├── providers/
├── ocr.rs
└── ...
```

To:

```
src/
├── url_to_text/
│   ├── fetchers/
│   ├── html/extractors/
│   └── text/
├── images_to_text/
├── converters/
├── pipelines/
└── ...
```

Total tasks: ~25 commits across 13 phases.

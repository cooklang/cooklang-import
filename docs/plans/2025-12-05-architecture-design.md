# Architecture Redesign

## Overview

Restructure cooklang-import to have clear separation between input processing, extraction, and conversion phases.

## Module Structure

```
src/
├── lib.rs                      # Public API exports
├── main.rs                     # CLI
├── model.rs                    # Recipe struct + .to_text_with_metadata()
├── builder.rs                  # Configuration, routing, fallback logic
├── config.rs                   # Config loading (extractors/converters order)
├── error.rs                    # Error types
│
├── pipelines/
│   ├── mod.rs
│   ├── url.rs                  # URL → Cooklang
│   ├── text.rs                 # Text → Cooklang
│   └── image.rs                # Images → Cooklang
│
├── url_to_text/
│   ├── mod.rs
│   ├── fetchers/
│   │   ├── mod.rs
│   │   ├── request.rs          # HTTP fetch (reqwest)
│   │   └── chrome.rs           # PAGE_SCRIBER_URL fetch
│   ├── html/
│   │   ├── mod.rs
│   │   └── extractors/
│   │       ├── mod.rs
│   │       ├── json_ld.rs
│   │       ├── microdata.rs
│   │       └── html_class.rs
│   └── text/
│       ├── mod.rs
│       └── extractor.rs        # LLM extraction for plain text
│
├── images_to_text/
│   ├── mod.rs
│   └── ocr.rs                  # Google Vision (path + base64)
│
└── converters/
    ├── mod.rs                  # Converter trait + factory
    ├── prompt.rs
    ├── open_ai.rs
    ├── anthropic.rs
    ├── azure_openai.rs
    ├── google.rs
    └── ollama.rs
```

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              URL INPUT                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌──────────────┐      ┌──────────────┐      ┌──────────────┐         │
│   │   Fetchers   │      │     HTML     │      │    Recipe    │         │
│   │   request/   │─────▶│  Extractors  │─────▶│     Obj      │────┐    │
│   │   chrome     │      │  (json_ld,   │      │              │    │    │
│   └──────────────┘      │  microdata,  │      └──────────────┘    │    │
│         │               │  html_class) │                          │    │
│         │               └──────────────┘                          │    │
│         │                                                         │    │
│         │  (if extractors fail)                                   │    │
│         ▼                                                         │    │
│   ┌──────────────┐      ┌──────────────┐                          │    │
│   │  Plain Text  │─────▶│     LLM      │──────────────────────────┼────┤
│   │  (from HTML  │      │  Extractor   │                          │    │
│   │   or Chrome) │      │              │                          │    │
│   └──────────────┘      └──────────────┘                          │    │
│                                                                   │    │
└───────────────────────────────────────────────────────────────────┼────┘
                                                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                             TEXT INPUT                                   │
├─────────────────────────────────────────────────────────────────────────┤
│   (assumed correctly formatted, optional LLM extraction)          │    │
│                                                                   │    │
└───────────────────────────────────────────────────────────────────┼────┘
                                                                    │
┌─────────────────────────────────────────────────────────────────────────┐
│                            IMAGE INPUT                                   │
├─────────────────────────────────────────────────────────────────────────┤
│   ┌──────────────┐                                                │    │
│   │     OCR      │────────────────────────────────────────────────┼────┤
│   │ Google Vision│                                                │    │
│   └──────────────┘                                                │    │
│                                                                   │    │
└───────────────────────────────────────────────────────────────────┼────┘
                                                                    │
                                                                    ▼
                                              ┌──────────────────────────┐
                                              │   Text with Frontmatter  │
                                              └────────────┬─────────────┘
                                                           │
                                                           ▼
                                              ┌──────────────────────────┐
                                              │       Converters         │
                                              └────────────┬─────────────┘
                                                           │
                                                           ▼
                                              ┌──────────────────────────┐
                                              │    Cooklang Output       │
                                              └──────────────────────────┘
```

## Intermediate Text Format

All paths produce this intermediate format before conversion:

```
---
title: Pasta Carbonara
source: https://example.com/recipe
author: Chef John
servings: 4
prep_time: 15 min
cook_time: 20 min
---

400g spaghetti
200g guanciale
4 egg yolks
100g pecorino romano

Cook pasta in salted water until al dente. While pasta cooks,
cut guanciale into small cubes and fry until crispy...
```

Rules:
- Frontmatter is optional (plain text/OCR paths may only have `source`)
- Ingredients: one per line, no blank lines between
- Blank line separates ingredients from instructions
- Instructions: free text, can have multiple paragraphs

## Recipe Model

```rust
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
        // Serialize to intermediate text format
    }
}
```

## Configuration

Extractors and converters are configurable:

```toml
[extractors]
enabled = ["json_ld", "microdata", "html_class"]
order = ["json_ld", "microdata", "html_class"]

[converters]
enabled = ["anthropic", "open_ai", "ollama"]
order = ["anthropic", "open_ai", "ollama"]
default = "anthropic"
```

## Input Sources

```rust
pub enum ImageSource {
    Path(String),
    Base64(String),
}

pub enum InputSource {
    Url(String),
    Text(String),
    Images(Vec<ImageSource>),
}
```

## Converters

Converters receive only ingredients + instructions (no frontmatter):

```rust
#[async_trait]
pub trait Converter: Send + Sync {
    fn name(&self) -> &str;
    async fn convert(&self, ingredients_and_instructions: &str) -> Result<String, Error>;
}
```

Final output assembly merges metadata (as YAML frontmatter) with converted Cooklang body.

## Final Output Format

```
---
title: Pasta Carbonara
source: https://example.com
author: Chef John
servings: 4
---

@spaghetti{400%g}
@guanciale{200%g}
@egg yolks{4}
@pecorino romano{100%g}

Cook @pasta in salted water until al dente. While pasta cooks,
cut @guanciale into small cubes and fry in #pan until crispy...
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP fetch failed: {0}")]
    FetchError(#[from] reqwest::Error),

    #[error("Chrome fetch failed: {0}")]
    ChromeFetchError(String),

    #[error("No extractor matched")]
    NoExtractorMatched,

    #[error("Extractor '{0}' failed: {1}")]
    ExtractorError(String, String),

    #[error("No converters available")]
    NoConvertersAvailable,

    #[error("Converter '{0}' failed: {1}")]
    ConverterError(String, String),

    #[error("All converters failed")]
    AllConvertersFailed,

    #[error("OCR failed: {0}")]
    OcrError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("No input source specified")]
    NoInputSource,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

## Key Decisions

1. **Shared fetchers** - `url_to_text/fetchers/` is shared between html and text paths with caching
2. **Configurable order** - Extractors and converters order defined in config
3. **Recipe serialization** - `.to_text_with_metadata()` method on Recipe struct
4. **Simple text format** - YAML frontmatter + ingredients (one per line) + blank line + instructions
5. **Converter input** - Only ingredients + instructions, not metadata
6. **Final output** - YAML frontmatter + Cooklang body
7. **Multiple images** - Supported via path or base64
8. **Fallback logic** - Handled in builder.rs

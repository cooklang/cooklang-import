# Image OCR Triple Extraction Design

## Overview

Change the image OCR architecture so that extracted text is processed by an LLM to produce a structured triple: **recipe text**, **metadata**, and **title** (via `RecipeComponents`).

## Current State

- Image OCR extracts raw text via Google Vision API
- Returns `RecipeComponents { text, metadata, name }` where:
  - `text` = raw OCR'd text
  - `metadata` = just "source: image.jpg"
  - `name` = empty string
- Structure (title, metadata, ingredients) only comes from downstream LLM converters

## New Architecture

### Data Flow

```
Image → Google Vision OCR → raw text → TextExtractor (LLM) → RecipeComponents {
    name: "Chocolate Cake",
    metadata: "source: image.jpg\nservings: 4\nprep_time: 15 min",
    text: "ingredients...\n\ninstructions..."
}
```

### LLM Prompt & JSON Schema

Updated prompt in `src/url_to_text/text/extractor.rs`:

```rust
const PROMPT: &str = r#"
You're an expert in extracting recipe information from messy texts (often OCR'd from images).
Sometimes the text is not a recipe - in that case specify that in the error field.

Given the text, output only this JSON without any other characters:

{
  "title": "<RECIPE TITLE OR NULL IF NOT FOUND>",
  "servings": "<SERVINGS AS STRING e.g. '4' or '4-6' OR NULL>",
  "prep_time": "<PREP TIME AS STRING e.g. '15 min' OR NULL>",
  "cook_time": "<COOK TIME AS STRING e.g. '30 min' OR NULL>",
  "total_time": "<TOTAL TIME AS STRING e.g. '45 min' OR NULL>",
  "ingredients": ["<LIST OF INGREDIENTS>"],
  "instructions": ["<LIST OF INSTRUCTIONS>"],
  "error": "<ERROR MESSAGE IF NO RECIPE, OTHERWISE NULL>"
}
"#;
```

**Key decisions:**
- Metadata fields are nullable strings (recipes often have partial info)
- Times kept as strings to preserve original format ("1 hour", "45 min", etc.)
- Title is separate from ingredients/instructions for clean separation

### Return Type Change

`TextExtractor::extract()` changes from returning `String` to returning `RecipeComponents`:

```rust
pub async fn extract(
    plain_text: &str,
    source: &str,
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>>
```

Parsing logic:
- Extract title → `RecipeComponents.name`
- Build metadata YAML from available fields (servings, prep_time, cook_time, total_time, source)
- Only include metadata fields that are present and non-empty
- Format ingredients and instructions as text → `RecipeComponents.text`

### Caller Updates

**`src/pipelines/url.rs`** - Simplify:
```rust
// Before:
let text_with_metadata = TextExtractor::extract(&plain_text, url).await?;
Ok(parse_text_to_components(&text_with_metadata))

// After:
TextExtractor::extract(&plain_text, url).await
```

**`src/pipelines/text.rs`** - Simplify:
```rust
// Before:
let extracted = TextExtractor::extract(text, "direct-input").await?;
Ok(parse_text_to_components(&extracted))

// After:
TextExtractor::extract(text, "direct-input").await
```

### Image Pipeline Integration

**`src/pipelines/image.rs`** - Add extraction with fallback:

```rust
pub async fn process(
    images: &[ImageSource],
) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
    // ... existing OCR loop ...

    let combined = all_text.join("\n\n");
    let source = sources.join(", ");

    // Try structured extraction if API key available
    if TextExtractor::is_available() {
        TextExtractor::extract(&combined, &source).await
    } else {
        // Fallback: return raw OCR text
        Ok(RecipeComponents {
            text: combined,
            metadata: format!("source: {}", source),
            name: String::new(),
        })
    }
}
```

**Behavior:**
- If `OPENAI_API_KEY` available → structured extraction with title, servings, etc.
- If not → raw OCR text (current behavior preserved)

### Test Updates

Update test mock in `fetch_json()`:

```rust
if api_key == "test_key" {
    return Ok(serde_json::json!({
        "title": "Test Recipe",
        "servings": "4",
        "prep_time": "10 min",
        "cook_time": "20 min",
        "total_time": "30 min",
        "ingredients": ["pasta", "sauce"],
        "instructions": ["Cook pasta with sauce"],
        "error": null
    }));
}
```

**Test coverage needed:**
- Title extraction (present and missing)
- Partial metadata (e.g., only servings, no times)
- All metadata fields present
- Error case (not a recipe)
- Image pipeline with and without API key

## Files Changed

| File | Change |
|------|--------|
| `src/url_to_text/text/extractor.rs` | Update prompt; return `RecipeComponents` |
| `src/pipelines/url.rs` | Remove `parse_text_to_components()` call |
| `src/pipelines/text.rs` | Remove `parse_text_to_components()` call |
| `src/pipelines/image.rs` | Add `TextExtractor` call with fallback |

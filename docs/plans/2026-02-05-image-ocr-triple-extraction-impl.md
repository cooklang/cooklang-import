# Image OCR Triple Extraction - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend `TextExtractor` to return `RecipeComponents` with title, metadata (servings, prep_time, cook_time, total_time), and structured recipe text.

**Architecture:** Update the LLM prompt to extract additional fields, change return type from `String` to `RecipeComponents`, update callers, and integrate into image pipeline with fallback.

**Tech Stack:** Rust, OpenAI API (gpt-4o-mini), tokio async, serde_json

---

### Task 1: Update TextExtractor Prompt and Return Type

**Files:**
- Modify: `src/url_to_text/text/extractor.rs`

**Step 1: Update the PROMPT constant**

Replace lines 6-16 with:

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

**Step 2: Add import for RecipeComponents**

Add at line 4:

```rust
use crate::pipelines::RecipeComponents;
```

**Step 3: Update extract() signature and implementation**

Replace lines 28-66 with:

```rust
    pub async fn extract(
        plain_text: &str,
        source: &str,
    ) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
        let json = fetch_json(plain_text.to_string()).await?;

        // Check for error (not a recipe)
        if let Some(error) = json["error"].as_str() {
            if !error.is_empty() {
                return Err(error.into());
            }
        }

        // Extract title (fallback to empty string)
        let name = json["title"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Build metadata YAML from available fields
        let mut metadata_lines = vec![format!("source: {}", source)];
        for field in ["servings", "prep_time", "cook_time", "total_time"] {
            if let Some(val) = json[field].as_str() {
                if !val.is_empty() {
                    metadata_lines.push(format!("{}: {}", field, val));
                }
            }
        }
        let metadata = metadata_lines.join("\n");

        // Format ingredients as newline-separated list
        let ingredients = json["ingredients"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join("\n");

        // Format instructions as space-separated (paragraph)
        let instructions = json["instructions"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|i| i.as_str().map(String::from))
            .collect::<Vec<String>>()
            .join(" ");

        // Combine ingredients and instructions
        let text = format!("{}\n\n{}", ingredients, instructions);

        Ok(RecipeComponents { text, metadata, name })
    }
```

**Step 4: Update the test mock in fetch_json()**

Replace the test mock (lines 73-78) with:

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

**Step 5: Verify compilation**

Run: `cargo check --lib`
Expected: No errors

**Step 6: Commit**

```bash
git add src/url_to_text/text/extractor.rs
git commit -m "feat: update TextExtractor to return RecipeComponents with title and metadata"
```

---

### Task 2: Update Text Pipeline Caller

**Files:**
- Modify: `src/pipelines/text.rs`

**Step 1: Simplify the extract branch**

Replace lines 9-12:

```rust
    if extract {
        // Run through LLM extractor
        let extracted = TextExtractor::extract(text, "direct-input").await?;
        Ok(parse_text_to_components(&extracted))
```

With:

```rust
    if extract {
        // Run through LLM extractor - returns RecipeComponents directly
        TextExtractor::extract(text, "direct-input").await
```

**Step 2: Verify compilation**

Run: `cargo check --lib`
Expected: No errors

**Step 3: Commit**

```bash
git add src/pipelines/text.rs
git commit -m "refactor: simplify text pipeline to use RecipeComponents from extractor"
```

---

### Task 3: Update URL Pipeline Caller

**Files:**
- Modify: `src/pipelines/url.rs`

**Step 1: Simplify the TextExtractor call**

Replace lines 67-71:

```rust
    // 5. Use TextExtractor to parse the plain text
    let text_with_metadata = TextExtractor::extract(&plain_text, url).await?;

    // Parse the text format and return as components
    Ok(parse_text_to_components(&text_with_metadata))
```

With:

```rust
    // 5. Use TextExtractor to parse the plain text - returns RecipeComponents directly
    TextExtractor::extract(&plain_text, url).await
```

**Step 2: Remove unused parse_text_to_components function**

Delete lines 108-127 (the `parse_text_to_components` function) since it's no longer used in this file.

**Step 3: Verify compilation**

Run: `cargo check --lib`
Expected: No errors

**Step 4: Commit**

```bash
git add src/pipelines/url.rs
git commit -m "refactor: simplify URL pipeline to use RecipeComponents from extractor"
```

---

### Task 4: Check for Unused parse_text_to_components in text.rs

**Files:**
- Modify: `src/pipelines/text.rs`

**Step 1: Check if parse_text_to_components is still used**

The non-extract branch still calls `parse_text_to_components`. Verify this function is still needed for the `else` branch (line 15-16).

**Step 2: No changes needed if still used**

The function is still used for the `extract: false` case. Keep it.

**Step 3: Run tests**

Run: `cargo test --lib`
Expected: All tests pass

**Step 4: Commit if any cleanup done**

If no changes needed, skip this commit.

---

### Task 5: Update Image Pipeline with TextExtractor Integration

**Files:**
- Modify: `src/pipelines/image.rs`

**Step 1: Add import for TextExtractor**

Add after line 2:

```rust
use crate::url_to_text::text::TextExtractor;
```

**Step 2: Update the process function**

Replace lines 20-28:

```rust
    let combined = all_text.join("\n\n");
    let source = sources.join(", ");

    Ok(RecipeComponents {
        text: combined,
        metadata: format!("source: {}", source),
        name: String::new(), // Images typically don't have a name extracted
    })
```

With:

```rust
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
```

**Step 3: Verify compilation**

Run: `cargo check --lib`
Expected: No errors

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/pipelines/image.rs
git commit -m "feat: integrate TextExtractor into image pipeline with fallback"
```

---

### Task 6: Add Unit Tests for New TextExtractor Behavior

**Files:**
- Modify: `src/url_to_text/text/extractor.rs`

**Step 1: Add test module at end of file**

Add after line 101:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_returns_recipe_components() {
        std::env::set_var("OPENAI_API_KEY", "test_key");

        let result = TextExtractor::extract("some recipe text", "test-source").await;

        assert!(result.is_ok());
        let components = result.unwrap();

        assert_eq!(components.name, "Test Recipe");
        assert!(components.metadata.contains("source: test-source"));
        assert!(components.metadata.contains("servings: 4"));
        assert!(components.metadata.contains("prep_time: 10 min"));
        assert!(components.metadata.contains("cook_time: 20 min"));
        assert!(components.metadata.contains("total_time: 30 min"));
        assert!(components.text.contains("pasta"));
        assert!(components.text.contains("sauce"));
        assert!(components.text.contains("Cook pasta with sauce"));
    }

    #[test]
    fn test_is_available_without_key() {
        std::env::remove_var("OPENAI_API_KEY");
        assert!(!TextExtractor::is_available());
    }

    #[test]
    fn test_is_available_with_key() {
        std::env::set_var("OPENAI_API_KEY", "test_key");
        assert!(TextExtractor::is_available());
    }
}
```

**Step 2: Run new tests**

Run: `cargo test extractor --lib`
Expected: All 3 tests pass

**Step 3: Commit**

```bash
git add src/url_to_text/text/extractor.rs
git commit -m "test: add unit tests for TextExtractor RecipeComponents output"
```

---

### Task 7: Run Full Test Suite and Fix Any Failures

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Fix any failures**

If any tests fail due to the changed return type, update them to work with `RecipeComponents` instead of `String`.

**Step 3: Commit fixes if any**

```bash
git add -A
git commit -m "fix: update tests for new TextExtractor return type"
```

---

### Task 8: Final Verification

**Step 1: Run clippy**

Run: `cargo clippy --lib`
Expected: No warnings

**Step 2: Run full test suite one more time**

Run: `cargo test`
Expected: All tests pass

**Step 3: Create summary commit if needed**

If there were any final tweaks:

```bash
git add -A
git commit -m "chore: final cleanup for image OCR triple extraction"
```

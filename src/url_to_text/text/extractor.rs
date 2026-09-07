use crate::pipelines::RecipeComponents;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::error::Error;

const PROMPT: &str = r#"
You're an expert in extracting recipe information from messy texts (often OCR'd from images).
Sometimes the text is not a recipe - in that case specify that in the error field.

IMPORTANT: Only extract information that is EXPLICITLY present in the text. Do NOT invent, guess, or estimate any values. If a field is not mentioned in the text, use null.

IMPORTANT: Keep the recipe's original language. Copy ingredients and instructions verbatim - never translate, paraphrase, or add wording of your own.

If the text contains only an ingredient list with no cooking instructions, return the ingredients and an empty instructions array - do NOT invent instructions.

Given the text, output only this JSON without any other characters:

{
  "title": "<RECIPE TITLE OR null IF NOT EXPLICITLY STATED>",
  "servings": "<SERVINGS OR null IF NOT EXPLICITLY STATED>",
  "prep_time": "<PREP TIME OR null IF NOT EXPLICITLY STATED>",
  "cook_time": "<COOK TIME OR null IF NOT EXPLICITLY STATED>",
  "total_time": "<TOTAL TIME OR null IF NOT EXPLICITLY STATED>",
  "ingredients": ["<LIST OF INGREDIENTS>"],
  "instructions": ["<LIST OF INSTRUCTIONS>"],
  "error": "<ERROR MESSAGE IF NO RECIPE, OTHERWISE null>"
}
"#;

const MODEL: &str = "gpt-4o-mini";

/// Upper bound on the text sent to the model.
///
/// The window is 128k tokens. OCR output and page text tokenize unevenly (CJK and
/// Cyrillic run well under 4 chars/token), so the cap leaves generous headroom
/// rather than betting on an average ratio. Six URL cookifications were rejected
/// outright in the week of 2026-08-17 for exceeding the window, one at 924,697
/// tokens.
pub(crate) const MAX_INPUT_CHARS: usize = 120_000;

/// Completion budget. Long recipes need room; without an explicit value the
/// response silently stops at the model default and comes back as invalid JSON.
const MAX_OUTPUT_TOKENS: u32 = 8_000;

/// Values models write into the `error` field to mean "no error". Three
/// cookifications in the week of 2026-08-17 were discarded because the model wrote
/// the *string* "null" next to a complete, usable recipe.
const NON_ERRORS: [&str; 6] = ["null", "none", "nil", "n/a", "na", "-"];

/// Clamp text to [`MAX_INPUT_CHARS`] on a character boundary.
pub(crate) fn clamp_input(text: &str) -> String {
    crate::pipelines::truncate_on_char_boundary(text.to_string(), MAX_INPUT_CHARS)
}

/// Pull the completion text out of an OpenAI chat response, distinguishing the
/// failure modes that used to surface as opaque serde errors.
pub(crate) fn response_content(response: &Value) -> Result<&str, Box<dyn Error + Send + Sync>> {
    let choice = &response["choices"][0];

    // A response cut off at the token limit is still valid JSON at the API level but
    // truncated JSON at the content level. Say so, instead of letting serde report
    // "EOF while parsing a string at line 23 column 11".
    if choice["finish_reason"].as_str() == Some("length") {
        return Err("Response truncated: the recipe exceeded the model's output limit".into());
    }

    match choice["message"]["content"].as_str() {
        Some(content) => Ok(content),
        None => {
            // Surface the API's own error message instead of a generic failure
            let detail = response["error"]["message"]
                .as_str()
                .unwrap_or("no content in response");
            Err(format!("Failed to get response content: {}", detail).into())
        }
    }
}

/// The extraction error the model reported, if it is a real one.
///
/// Returns `None` when the field holds a "no error" sentinel, or when the model
/// flagged a problem but still returned usable content - a partial recipe beats no
/// recipe.
pub(crate) fn extraction_error(json: &Value) -> Option<String> {
    let error = json["error"].as_str()?.trim();
    if error.is_empty() || NON_ERRORS.contains(&error.to_ascii_lowercase().as_str()) {
        return None;
    }

    let has = |key: &str| {
        json[key].as_array().is_some_and(|a| {
            a.iter()
                .any(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
        })
    };
    if has("ingredients") || has("instructions") {
        return None;
    }

    Some(error.to_string())
}

pub struct TextExtractor;

impl TextExtractor {
    /// Check if the TextExtractor is available (has required API key configured)
    pub fn is_available() -> bool {
        env::var("OPENAI_API_KEY").is_ok()
    }

    pub async fn extract(
        plain_text: &str,
        source: &str,
    ) -> Result<RecipeComponents, Box<dyn Error + Send + Sync>> {
        let json = fetch_json(plain_text.to_string()).await?;

        // Check for error (not a recipe)
        if let Some(error) = extraction_error(&json) {
            return Err(error.into());
        }

        // Extract title (fallback to empty string)
        let name = json["title"].as_str().unwrap_or("").to_string();

        // Build metadata YAML from available fields, mapping the extractor's JSON
        // field names to canonical Cooklang metadata keys.
        let mut entries = vec![("source".to_string(), source.to_string())];
        if let Some(val) = json["servings"].as_str() {
            entries.extend(crate::pipelines::servings_entries(val));
        }
        for (field, key) in [
            ("prep_time", "prep time"),
            ("cook_time", "cook time"),
            ("total_time", "time required"),
        ] {
            if let Some(val) = json[field].as_str() {
                if !val.is_empty() {
                    entries.push((key.to_string(), val.to_string()));
                }
            }
        }
        let metadata = crate::pipelines::metadata_to_yaml(&entries);

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

        Ok(RecipeComponents {
            text,
            metadata,
            name,
        })
    }
}

async fn fetch_json(texts: String) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let api_key = env::var("OPENAI_API_KEY")?;

    // For testing environment, return mock data
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

    let texts = clamp_input(&texts);

    let response = Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "response_format": { "type": "json_object" },
            "max_tokens": MAX_OUTPUT_TOKENS,
            "messages": [
                { "role": "system", "content": PROMPT },
                { "role": "user", "content": texts }
            ]
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let content = response_content(&response)?;

    serde_json::from_str(content).map_err(|e| {
        format!("Model returned malformed JSON ({e}); the recipe could not be read").into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// OPENAI_API_KEY is process-global, so the tests that mutate it must not run
    /// concurrently with each other.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // --- Regression: truncated LLM responses (production failures 2026-08-17..24) ---
    //
    // 9 image cookifications died with "EOF while parsing a string at line 23
    // column 11": the model hit its completion cap, OpenAI returned partial JSON with
    // finish_reason "length", and serde was handed a half-written string. The parse
    // error told nobody what actually happened.

    #[test]
    fn test_truncated_response_reports_the_real_cause() {
        let response = serde_json::json!({
            "choices": [{
                "finish_reason": "length",
                "message": { "content": "{\n  \"title\": \"Pasta\",\n  \"ingredients\": [\"pas" }
            }]
        });
        let err = response_content(&response).unwrap_err().to_string();
        assert!(
            err.contains("truncated"),
            "expected a truncation message, got: {err}"
        );
        assert!(
            !err.contains("EOF while parsing"),
            "leaked the serde error: {err}"
        );
    }

    #[test]
    fn test_complete_response_returns_content() {
        let response = serde_json::json!({
            "choices": [{ "finish_reason": "stop", "message": { "content": "{\"title\":\"Pasta\"}" } }]
        });
        assert_eq!(
            response_content(&response).unwrap(),
            "{\"title\":\"Pasta\"}"
        );
    }

    #[test]
    fn test_api_error_message_is_surfaced() {
        let response = serde_json::json!({
            "error": { "message": "Rate limit reached for gpt-4o-mini" }
        });
        let err = response_content(&response).unwrap_err().to_string();
        assert!(err.contains("Rate limit reached"), "got: {err}");
    }

    // --- Regression: literal "null" in the error field ---
    //
    // 3 cookifications failed with the message "null". The model had filled in a
    // perfectly good recipe but wrote the *string* "null" into the error field, and
    // the guard threw the extraction away.

    #[test]
    fn test_literal_null_error_string_is_not_an_error() {
        for sentinel in ["null", "NULL", "None", "none", "nil", "N/A", "", "  "] {
            let json = serde_json::json!({
                "title": "Pasta",
                "ingredients": ["200 g pasta"],
                "instructions": ["Boil it."],
                "error": sentinel,
            });
            assert!(
                extraction_error(&json).is_none(),
                "sentinel {sentinel:?} must not be treated as a real error"
            );
        }
    }

    #[test]
    fn test_real_error_with_no_content_is_an_error() {
        let json = serde_json::json!({
            "title": null,
            "ingredients": [],
            "instructions": [],
            "error": "No recipe found in the text",
        });
        assert_eq!(
            extraction_error(&json).as_deref(),
            Some("No recipe found in the text")
        );
    }

    #[test]
    fn test_error_alongside_extracted_content_is_ignored() {
        // If the model both flagged an error and returned a usable recipe, keep the
        // recipe - throwing it away is strictly worse for the user.
        let json = serde_json::json!({
            "title": "Tomato Soup",
            "ingredients": ["500 g tomatoes", "1 onion"],
            "instructions": ["Simmer for 20 minutes."],
            "error": "text was partially unreadable",
        });
        assert!(extraction_error(&json).is_none());
    }

    #[test]
    fn test_missing_error_field_is_not_an_error() {
        let json = serde_json::json!({ "title": "X", "ingredients": ["a"], "instructions": [] });
        assert!(extraction_error(&json).is_none());
    }

    // --- Regression: oversized input ---
    //
    // 6 URL cookifications exceeded the 128k context window, one at 924,697 tokens.

    #[test]
    fn test_input_is_truncated_before_send() {
        let huge = "word ".repeat(200_000);
        let clipped = clamp_input(&huge);
        assert!(
            clipped.len() <= MAX_INPUT_CHARS,
            "got {} chars",
            clipped.len()
        );
    }

    #[test]
    fn test_short_input_is_untouched() {
        assert_eq!(clamp_input("2 eggs, 1 cup flour"), "2 eggs, 1 cup flour");
    }

    #[test]
    fn test_truncation_respects_char_boundaries() {
        // Multi-byte input must not be sliced mid-character.
        let huge = "ガッツリ食べたいご飯が進む味".repeat(50_000);
        let clipped = clamp_input(&huge);
        assert!(clipped.len() <= MAX_INPUT_CHARS);
        assert!(huge.starts_with(&clipped));
    }

    // Holding the guard across the await is intentional: the mocked extract() call
    // must see the key this test set, and each test runs as a single task, so there
    // is no deadlock to introduce.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_extract_returns_recipe_components() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OPENAI_API_KEY", "test_key");

        let result = TextExtractor::extract("some recipe text", "test-source").await;

        assert!(result.is_ok());
        let components = result.unwrap();

        assert_eq!(components.name, "Test Recipe");
        assert!(components.metadata.contains("source: test-source"));
        assert!(components.metadata.contains("servings: 4"));
        assert!(components.metadata.contains("prep time: 10 min"));
        assert!(components.metadata.contains("cook time: 20 min"));
        assert!(components.metadata.contains("time required: 30 min"));
        assert!(components.text.contains("pasta"));
        assert!(components.text.contains("sauce"));
        assert!(components.text.contains("Cook pasta with sauce"));
    }

    #[test]
    fn test_is_available_without_key() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("OPENAI_API_KEY");
        assert!(!TextExtractor::is_available());
    }

    #[test]
    fn test_is_available_with_key() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OPENAI_API_KEY", "test_key");
        assert!(TextExtractor::is_available());
    }
}

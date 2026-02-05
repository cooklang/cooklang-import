use reqwest::Client;
use serde_json::Value;
use std::env;
use std::error::Error;
use crate::pipelines::RecipeComponents;

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

const MODEL: &str = "gpt-4o-mini";

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

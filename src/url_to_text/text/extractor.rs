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
    /// Check if the TextExtractor is available (has required API key configured)
    pub fn is_available() -> bool {
        env::var("OPENAI_API_KEY").is_ok()
    }

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

    // For testing environment, return mock data
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

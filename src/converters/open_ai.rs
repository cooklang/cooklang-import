use crate::config::ProviderConfig;
use super::{Converter, COOKLANG_CONVERTER_PROMPT};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct OpenAiConverter {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl OpenAiConverter {
    /// Create a new OpenAI converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or("OPENAI_API_KEY not found in config or environment")?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com".to_string());

        Ok(OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }

    /// Create a new OpenAI converter from environment variables
    ///
    /// Uses OPENAI_API_KEY and OPENAI_MODEL (defaults to gpt-4.1-mini) from environment
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());

        Ok(OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com".to_string(),
            model,
            temperature: 0.9,
            max_tokens: 2000,
        })
    }

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, base_url: String, model: String) -> Self {
        OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url,
            model,
            temperature: 0.9,
            max_tokens: 2000,
        }
    }
}

#[async_trait]
impl Converter for OpenAiConverter {
    fn name(&self) -> &str {
        "open_ai"
    }

    async fn convert(&self, content: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept-Encoding", "identity")
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": COOKLANG_CONVERTER_PROMPT},
                    {"role": "user", "content": content}
                ],
                "temperature": self.temperature,
                "max_tokens": self.max_tokens,
                "stream": false
            }))
            .send()
            .await?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body (status {}): {}", status, e))?;
        debug!("Raw response: {}", response_text);

        let response_body: Value = serde_json::from_str(&response_text).map_err(|e| {
            format!(
                "Failed to parse JSON: {}. Raw response: {}",
                e,
                &response_text[..response_text.len().min(500)]
            )
        })?;

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_msg = error["message"].as_str().unwrap_or("Unknown API error");
            return Err(format!("OpenAI API error: {}", error_msg).into());
        }

        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from response. Response: {}",
                    serde_json::to_string_pretty(&response_body)
                        .unwrap_or_else(|_| "unparseable".to_string())
                )
            })?
            .to_string();

        Ok(cooklang_recipe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_convert() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "choices": [{
                        "message": {
                            "content": ">> ingredients\n@pasta{500%g}\n@sauce\n\n>> instructions\n1. Cook pasta\n2. Add sauce"
                        }
                    }]
                }"#,
            )
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "pasta\nsauce\n\nCook pasta with sauce";

        let result = converter.convert(content).await.unwrap();
        assert!(result.contains("@pasta"));
        assert!(result.contains("@sauce"));
        mock.assert();
    }

    #[tokio::test]
    async fn test_convert_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid request"}"#)
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "ingredient\n\nstep";

        let result = converter.convert(content).await;
        assert!(result.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_converter_name() {
        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            "https://api.openai.com".to_string(),
            "gpt-4.1-mini".to_string(),
        );
        assert_eq!(converter.name(), "open_ai");
    }
}

use crate::config::ProviderConfig;
use crate::providers::{build_converter_prompt, LlmProvider};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider from configuration
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

        Ok(OpenAIProvider {
            client: Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }

    /// Create a new OpenAI provider from environment variables
    ///
    /// Uses OPENAI_API_KEY and OPENAI_MODEL (defaults to gpt-4.1-mini) from environment
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());

        Ok(OpenAIProvider {
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
        OpenAIProvider {
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
impl LlmProvider for OpenAIProvider {
    fn provider_name(&self) -> &str {
        "openai"
    }

    async fn convert(
        &self,
        content: &str,
        recipe_language: Option<&str>,
    ) -> Result<String, Box<dyn Error>> {
        let system_prompt = build_converter_prompt(recipe_language);

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": content}
                ],
                "temperature": self.temperature,
                "max_tokens": self.max_tokens
            }))
            .send()
            .await?;

        let response_body: Value = response.json().await?;
        debug!("{:?}", response_body);
        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("Failed to extract content from response")?
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

        let converter = OpenAIProvider::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "pasta\nsauce\n\nCook pasta with sauce";

        let result = converter.convert(content, None).await.unwrap();
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

        let converter = OpenAIProvider::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "ingredient\n\nstep";

        let result = converter.convert(content, None).await;
        assert!(result.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_provider_name() {
        let provider = OpenAIProvider::with_base_url(
            "fake_api_key".to_string(),
            "https://api.openai.com".to_string(),
            "gpt-4.1-mini".to_string(),
        );
        assert_eq!(provider.provider_name(), "openai");
    }
}

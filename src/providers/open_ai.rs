use crate::config::ProviderConfig;
use crate::providers::{LlmProvider, COOKLANG_CONVERTER_PROMPT};
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

    /// Create a new OpenAI provider with simple parameters (for backward compatibility)
    pub fn with_api_key(api_key: String, model: String) -> Self {
        OpenAIProvider {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com".to_string(),
            model,
            temperature: 0.7,
            max_tokens: 2000,
        }
    }

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, base_url: String, model: String) -> Self {
        OpenAIProvider {
            client: Client::new(),
            api_key,
            base_url,
            model,
            temperature: 0.7,
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
        ingredients: &str,
        instructions: &str,
    ) -> Result<String, Box<dyn Error>> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": COOKLANG_CONVERTER_PROMPT},
                    {"role": "user", "content": format!("Ingredients: {:?}\nInstructions: {}", ingredients, instructions)}
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
        let ingredients = "pasta\nsauce";
        let instructions = "Cook pasta with sauce";

        let result = converter.convert(ingredients, instructions).await.unwrap();
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
        let ingredients = "ingredient";
        let instructions = "step";

        let result = converter.convert(ingredients, instructions).await;
        assert!(result.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_provider_name() {
        let provider =
            OpenAIProvider::with_api_key("fake_api_key".to_string(), "gpt-4".to_string());
        assert_eq!(provider.provider_name(), "openai");
    }
}

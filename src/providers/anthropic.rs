use crate::config::ProviderConfig;
use crate::providers::{build_converter_prompt, LlmProvider};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or("ANTHROPIC_API_KEY not found in config or environment")?;

        Ok(AnthropicProvider {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, _base_url: String, model: String) -> Self {
        AnthropicProvider {
            client: Client::builder().build().unwrap_or_else(|_| Client::new()),
            api_key,
            model,
            temperature: 0.7,
            max_tokens: 4000,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn provider_name(&self) -> &str {
        "anthropic"
    }

    async fn convert(
        &self,
        content: &str,
        recipe_language: Option<&str>,
    ) -> Result<String, Box<dyn Error>> {
        let system_prompt = build_converter_prompt(recipe_language);

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "temperature": self.temperature,
                "system": system_prompt,
                "messages": [
                    {
                        "role": "user",
                        "content": content
                    }
                ]
            }))
            .send()
            .await?;

        let response_body: Value = response.json().await?;
        debug!("{:?}", response_body);

        let cooklang_recipe = response_body["content"][0]["text"]
            .as_str()
            .ok_or("Failed to extract content from Anthropic response")?
            .to_string();

        Ok(cooklang_recipe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anthropic_convert() {
        // Note: We can't easily test with base_url for Anthropic since it's hardcoded
        // This test would require modifying the AnthropicProvider to accept a custom base URL
        // For now, we just test that the provider can be created
        let config = ProviderConfig {
            enabled: true,
            model: "claude-sonnet-4.5".to_string(),
            temperature: 0.7,
            max_tokens: 4000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        let provider = AnthropicProvider::new(&config);
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_provider_name() {
        let config = ProviderConfig {
            enabled: true,
            model: "claude-sonnet-4.5".to_string(),
            temperature: 0.7,
            max_tokens: 4000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        let provider = AnthropicProvider::new(&config).unwrap();
        assert_eq!(provider.provider_name(), "anthropic");
    }
}

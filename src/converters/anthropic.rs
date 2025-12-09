use super::{inject_recipe, ConversionMetadata, ConversionResult, Converter, TokenUsage};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Instant;

pub struct AnthropicConverter {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl AnthropicConverter {
    /// Create a new Anthropic converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or("ANTHROPIC_API_KEY not found in config or environment")?;

        Ok(AnthropicConverter {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, _base_url: String, model: String) -> Self {
        AnthropicConverter {
            client: Client::builder().build().unwrap_or_else(|_| Client::new()),
            api_key,
            model,
            temperature: 0.7,
            max_tokens: 4000,
        }
    }
}

#[async_trait]
impl Converter for AnthropicConverter {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn convert(
        &self,
        content: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "temperature": self.temperature,
                "messages": [
                    {
                        "role": "user",
                        "content": inject_recipe(content)
                    }
                ]
            }))
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let response_body: Value = response.json().await?;
        debug!("Anthropic response: {:?}", response_body);

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_type = error["type"].as_str().unwrap_or("unknown");
            let error_message = error["message"].as_str().unwrap_or("Unknown error");
            return Err(format!("Anthropic API error ({}): {}", error_type, error_message).into());
        }

        let cooklang_recipe = response_body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from Anthropic response. Response: {}",
                    serde_json::to_string_pretty(&response_body)
                        .unwrap_or_else(|_| response_body.to_string())
                )
            })?
            .to_string();

        // Extract metadata from response
        let model_version = response_body["model"].as_str().map(|s| s.to_string());
        let input_tokens = response_body["usage"]["input_tokens"]
            .as_u64()
            .map(|v| v as u32);
        let output_tokens = response_body["usage"]["output_tokens"]
            .as_u64()
            .map(|v| v as u32);

        Ok(ConversionResult {
            content: cooklang_recipe,
            metadata: ConversionMetadata {
                model_version,
                tokens_used: TokenUsage {
                    input_tokens,
                    output_tokens,
                },
                latency_ms,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anthropic_convert() {
        // Note: We can't easily test with base_url for Anthropic since it's hardcoded
        // This test would require modifying the AnthropicConverter to accept a custom base URL
        // For now, we just test that the converter can be created
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

        let converter = AnthropicConverter::new(&config);
        assert!(converter.is_ok());
    }

    #[tokio::test]
    async fn test_converter_name() {
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

        let converter = AnthropicConverter::new(&config).unwrap();
        assert_eq!(converter.name(), "anthropic");
    }
}

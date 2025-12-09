use super::{inject_recipe, ConversionMetadata, ConversionResult, Converter, TokenUsage};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Instant;

pub struct OllamaConverter {
    client: Client,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl OllamaConverter {
    /// Create a new Ollama converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        Ok(OllamaConverter {
            client: Client::new(),
            base_url,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }

    #[doc(hidden)]
    pub fn with_base_url(base_url: String, model: String) -> Self {
        OllamaConverter {
            client: Client::new(),
            base_url,
            model,
            temperature: 0.7,
            max_tokens: 2000,
        }
    }
}

#[async_trait]
impl Converter for OllamaConverter {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn convert(
        &self,
        content: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        // Ollama uses OpenAI-compatible API
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "user", "content": inject_recipe(content)}
                ],
                "temperature": self.temperature,
                "max_tokens": self.max_tokens
            }))
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let response_body: Value = response.json().await?;
        debug!("Ollama response: {:?}", response_body);

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_message = error
                .as_str()
                .unwrap_or_else(|| error["message"].as_str().unwrap_or("Unknown error"));
            return Err(format!("Ollama API error: {}", error_message).into());
        }

        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from Ollama response. Response: {}",
                    serde_json::to_string_pretty(&response_body)
                        .unwrap_or_else(|_| response_body.to_string())
                )
            })?
            .to_string();

        // Extract metadata from response (OpenAI-compatible format)
        let model_version = response_body["model"].as_str().map(|s| s.to_string());
        let input_tokens = response_body["usage"]["prompt_tokens"]
            .as_u64()
            .map(|v| v as u32);
        let output_tokens = response_body["usage"]["completion_tokens"]
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
    use mockito::Server;

    #[tokio::test]
    async fn test_ollama_convert() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "choices": [{
                        "message": {
                            "content": "Cook @pasta{500%g} and add @sauce"
                        }
                    }]
                }"#,
            )
            .create();

        let converter = OllamaConverter::with_base_url(server.url(), "llama3".to_string());
        let content = "pasta\nsauce\n\nCook pasta with sauce";

        let result = converter.convert(content).await.unwrap();
        assert!(result.content.contains("@pasta"));
        assert!(result.content.contains("@sauce"));
        mock.assert();
    }

    #[tokio::test]
    async fn test_converter_name() {
        let config = ProviderConfig {
            enabled: true,
            model: "llama3".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        let converter = OllamaConverter::new(&config).unwrap();
        assert_eq!(converter.name(), "ollama");
    }

    #[tokio::test]
    async fn test_default_base_url() {
        let config = ProviderConfig {
            enabled: true,
            model: "llama3".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: None,
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        let converter = OllamaConverter::new(&config).unwrap();
        assert_eq!(converter.base_url, "http://localhost:11434");
    }
}

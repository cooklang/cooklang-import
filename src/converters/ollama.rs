use super::{Converter, COOKLANG_CONVERTER_PROMPT};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

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

    async fn convert(&self, content: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Ollama uses OpenAI-compatible API
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": COOKLANG_CONVERTER_PROMPT},
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
            .ok_or("Failed to extract content from Ollama response")?
            .to_string();

        Ok(cooklang_recipe)
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
        assert!(result.contains("@pasta"));
        assert!(result.contains("@sauce"));
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

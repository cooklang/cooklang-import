use super::{inject_recipe, ConversionMetadata, ConversionResult, Converter, TokenUsage};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Instant;

pub struct AzureOpenAiConverter {
    client: Client,
    api_key: String,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    temperature: f32,
    max_tokens: u32,
}

impl AzureOpenAiConverter {
    /// Create a new Azure OpenAI converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("AZURE_OPENAI_API_KEY").ok())
            .ok_or("AZURE_OPENAI_API_KEY not found in config or environment")?;

        let endpoint = config
            .endpoint
            .clone()
            .ok_or("Azure OpenAI endpoint is required")?;

        let deployment_name = config
            .deployment_name
            .clone()
            .ok_or("Azure OpenAI deployment_name is required")?;

        let api_version = config
            .api_version
            .clone()
            .unwrap_or_else(|| "2024-02-15-preview".to_string());

        Ok(AzureOpenAiConverter {
            client: Client::new(),
            api_key,
            endpoint,
            deployment_name,
            api_version,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }
}

#[async_trait]
impl Converter for AzureOpenAiConverter {
    fn name(&self) -> &str {
        "azure_openai"
    }

    async fn convert(
        &self,
        content: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        // Azure OpenAI URL format:
        // https://{endpoint}/openai/deployments/{deployment-name}/chat/completions?api-version={api-version}
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint.trim_end_matches('/'),
            self.deployment_name,
            self.api_version
        );

        let response = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .json(&json!({
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
        debug!("Azure OpenAI response: {:?}", response_body);

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_code = error["code"].as_str().unwrap_or("unknown");
            let error_message = error["message"].as_str().unwrap_or("Unknown error");
            return Err(
                format!("Azure OpenAI API error ({}): {}", error_code, error_message).into(),
            );
        }

        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from Azure OpenAI response. Response: {}",
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
    async fn test_converter_name() {
        let config = ProviderConfig {
            enabled: true,
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: Some("https://test.openai.azure.com".to_string()),
            deployment_name: Some("gpt-4".to_string()),
            api_version: Some("2024-02-15-preview".to_string()),
            project_id: None,
        };

        let converter = AzureOpenAiConverter::new(&config).unwrap();
        assert_eq!(converter.name(), "azure_openai");
    }

    #[tokio::test]
    async fn test_azure_convert() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock(
                "POST",
                "/openai/deployments/gpt-4/chat/completions?api-version=2024-02-15-preview",
            )
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

        let config = ProviderConfig {
            enabled: true,
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: Some(server.url()),
            deployment_name: Some("gpt-4".to_string()),
            api_version: Some("2024-02-15-preview".to_string()),
            project_id: None,
        };

        let converter = AzureOpenAiConverter::new(&config).unwrap();
        let content = "pasta\nsauce\n\nCook pasta with sauce";

        let result = converter.convert(content).await.unwrap();
        assert!(result.content.contains("@pasta"));
        assert!(result.content.contains("@sauce"));
        mock.assert();
    }
}

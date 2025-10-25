use crate::config::ProviderConfig;
use crate::providers::{LlmProvider, COOKLANG_CONVERTER_PROMPT};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct AzureOpenAIProvider {
    client: Client,
    api_key: String,
    endpoint: String,
    deployment_name: String,
    api_version: String,
    temperature: f32,
    max_tokens: u32,
}

impl AzureOpenAIProvider {
    /// Create a new Azure OpenAI provider from configuration
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

        Ok(AzureOpenAIProvider {
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
impl LlmProvider for AzureOpenAIProvider {
    fn provider_name(&self) -> &str {
        "azure_openai"
    }

    async fn convert(
        &self,
        ingredients: &str,
        instructions: &str,
    ) -> Result<String, Box<dyn Error>> {
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
    async fn test_provider_name() {
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

        let provider = AzureOpenAIProvider::new(&config).unwrap();
        assert_eq!(provider.provider_name(), "azure_openai");
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

        let provider = AzureOpenAIProvider::new(&config).unwrap();
        let ingredients = "pasta\nsauce";
        let instructions = "Cook pasta with sauce";

        let result = provider.convert(ingredients, instructions).await.unwrap();
        assert!(result.contains("@pasta"));
        assert!(result.contains("@sauce"));
        mock.assert();
    }
}

use crate::config::ProviderConfig;
use crate::providers::{LlmProvider, COOKLANG_CONVERTER_PROMPT};
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;

pub struct GoogleProvider {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl GoogleProvider {
    /// Create a new Google Gemini provider from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .ok_or("GOOGLE_API_KEY not found in config or environment")?;

        Ok(GoogleProvider {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }
}

#[async_trait]
impl LlmProvider for GoogleProvider {
    fn provider_name(&self) -> &str {
        "google"
    }

    async fn convert(&self, content: &str) -> Result<String, Box<dyn Error>> {
        // Google Gemini API endpoint
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .json(&json!({
                "contents": [{
                    "parts": [{
                        "text": format!(
                            "{}\n\n{}",
                            COOKLANG_CONVERTER_PROMPT,
                            content
                        )
                    }]
                }],
                "generationConfig": {
                    "temperature": self.temperature,
                    "maxOutputTokens": self.max_tokens
                }
            }))
            .send()
            .await?;

        let response_body: Value = response.json().await?;
        debug!("{:?}", response_body);

        let cooklang_recipe = response_body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or("Failed to extract content from Google Gemini response")?
            .to_string();

        Ok(cooklang_recipe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_name() {
        let config = ProviderConfig {
            enabled: true,
            model: "gemini-2.5-flash".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        let provider = GoogleProvider::new(&config).unwrap();
        assert_eq!(provider.provider_name(), "google");
    }
}

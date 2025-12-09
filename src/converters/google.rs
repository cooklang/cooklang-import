use super::{inject_recipe, ConversionMetadata, ConversionResult, Converter, TokenUsage};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Instant;

pub struct GoogleConverter {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
}

impl GoogleConverter {
    /// Create a new Google Gemini converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .ok_or("GOOGLE_API_KEY not found in config or environment")?;

        Ok(GoogleConverter {
            client: Client::new(),
            api_key,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        })
    }
}

#[async_trait]
impl Converter for GoogleConverter {
    fn name(&self) -> &str {
        "google"
    }

    async fn convert(
        &self,
        content: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

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
                        "text": inject_recipe(content)
                    }]
                }],
                "generationConfig": {
                    "temperature": self.temperature,
                    "maxOutputTokens": self.max_tokens
                }
            }))
            .send()
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let response_body: Value = response.json().await?;
        debug!("Google Gemini response: {:?}", response_body);

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_code = error["code"].as_i64().unwrap_or(0);
            let error_message = error["message"].as_str().unwrap_or("Unknown error");
            return Err(format!(
                "Google Gemini API error ({}): {}",
                error_code, error_message
            )
            .into());
        }

        let cooklang_recipe = response_body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from Google Gemini response. Response: {}",
                    serde_json::to_string_pretty(&response_body)
                        .unwrap_or_else(|_| response_body.to_string())
                )
            })?
            .to_string();

        // Extract metadata from response
        // Google returns modelVersion and usageMetadata
        let model_version = response_body["modelVersion"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| Some(self.model.clone()));
        let input_tokens = response_body["usageMetadata"]["promptTokenCount"]
            .as_u64()
            .map(|v| v as u32);
        let output_tokens = response_body["usageMetadata"]["candidatesTokenCount"]
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
    async fn test_converter_name() {
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

        let converter = GoogleConverter::new(&config).unwrap();
        assert_eq!(converter.name(), "google");
    }
}

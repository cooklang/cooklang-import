use crate::config::AiConfig;
use crate::providers::{LlmProvider, ProviderFactory};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

pub struct FallbackProvider {
    providers: Vec<Box<dyn LlmProvider>>,
    retry_attempts: u32,
    retry_delay_ms: u64,
}

impl FallbackProvider {
    /// Create a new fallback provider from configuration
    pub fn new(config: &AiConfig) -> Result<Self, Box<dyn Error>> {
        if !config.fallback.enabled {
            // If fallback is disabled, just use the default provider
            let default_provider = ProviderFactory::get_default_provider(config)?;
            return Ok(FallbackProvider {
                providers: vec![default_provider],
                retry_attempts: 1,
                retry_delay_ms: 0,
            });
        }

        let mut providers = Vec::new();

        // Create providers in fallback order
        for provider_name in &config.fallback.order {
            if let Some(provider_config) = config.providers.get(provider_name) {
                if provider_config.enabled {
                    match ProviderFactory::create(provider_name, provider_config) {
                        Ok(provider) => {
                            info!("Added '{}' to fallback chain", provider_name);
                            providers.push(provider);
                        }
                        Err(e) => {
                            warn!("Failed to initialize provider '{}': {}", provider_name, e);
                        }
                    }
                }
            } else {
                warn!(
                    "Provider '{}' in fallback order not found in configuration",
                    provider_name
                );
            }
        }

        if providers.is_empty() {
            return Err("No providers available in fallback configuration".into());
        }

        Ok(FallbackProvider {
            providers,
            retry_attempts: config.fallback.retry_attempts,
            retry_delay_ms: config.fallback.retry_delay_ms,
        })
    }

    /// Try a provider with exponential backoff retry logic
    async fn try_provider_with_retry(
        &self,
        provider: &dyn LlmProvider,
        content: &str,
    ) -> Result<String, String> {
        let mut last_error = None;

        for attempt in 1..=self.retry_attempts {
            debug!(
                "Attempting conversion with {} (attempt {}/{})",
                provider.provider_name(),
                attempt,
                self.retry_attempts
            );

            let should_retry = {
                let result = provider.convert(content).await;

                match result {
                    Ok(result) => {
                        info!(
                            "Successfully converted recipe using {}",
                            provider.provider_name()
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        // Convert error to string immediately to avoid Send issues
                        let error_msg = format!("{}", e);

                        warn!(
                            "Provider {} failed (attempt {}/{}): {}",
                            provider.provider_name(),
                            attempt,
                            self.retry_attempts,
                            error_msg
                        );
                        last_error = Some(error_msg);
                        attempt < self.retry_attempts
                    }
                }
            };

            // Sleep only if we need to retry
            if should_retry {
                // Exponential backoff: delay increases with each attempt
                let delay = Duration::from_millis(self.retry_delay_ms * attempt as u64);
                debug!("Waiting {:?} before retry", delay);
                sleep(delay).await;
            }
        }

        Err(last_error.unwrap())
    }
}

#[async_trait]
impl LlmProvider for FallbackProvider {
    fn provider_name(&self) -> &str {
        "fallback"
    }

    async fn convert(&self, content: &str) -> Result<String, Box<dyn Error>> {
        let mut all_errors: Vec<String> = Vec::new();

        for provider in &self.providers {
            match self
                .try_provider_with_retry(provider.as_ref(), content)
                .await
            {
                Ok(result) => return Ok(result),
                Err(e) => {
                    all_errors.push(format!("{}: {}", provider.provider_name(), e));
                }
            }
        }

        Err(format!("All providers failed:\n{}", all_errors.join("\n")).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FallbackConfig, ProviderConfig};
    use std::collections::HashMap;

    fn create_test_config_with_fallback() -> AiConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                enabled: true,
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_tokens: 2000,
                api_key: Some("test-key".to_string()),
                base_url: None,
                endpoint: None,
                deployment_name: None,
                api_version: None,
                project_id: None,
            },
        );

        AiConfig {
            default_provider: "openai".to_string(),
            providers,
            fallback: FallbackConfig {
                enabled: true,
                order: vec!["openai".to_string()],
                retry_attempts: 3,
                retry_delay_ms: 100,
            },
        }
    }

    #[tokio::test]
    async fn test_fallback_provider_creation() {
        let config = create_test_config_with_fallback();
        let fallback = FallbackProvider::new(&config);
        assert!(fallback.is_ok());
    }

    #[tokio::test]
    async fn test_fallback_provider_name() {
        let config = create_test_config_with_fallback();
        let fallback = FallbackProvider::new(&config).unwrap();
        assert_eq!(fallback.provider_name(), "fallback");
    }

    #[tokio::test]
    async fn test_fallback_disabled() {
        let mut config = create_test_config_with_fallback();
        config.fallback.enabled = false;

        let fallback = FallbackProvider::new(&config).unwrap();
        // With fallback disabled, only one provider should be in the list
        assert_eq!(fallback.providers.len(), 1);
        assert_eq!(fallback.retry_attempts, 1);
    }

    #[tokio::test]
    async fn test_fallback_no_providers() {
        let config = AiConfig {
            default_provider: "openai".to_string(),
            providers: HashMap::new(),
            fallback: FallbackConfig {
                enabled: true,
                order: vec!["openai".to_string()],
                retry_attempts: 3,
                retry_delay_ms: 100,
            },
        };

        let result = FallbackProvider::new(&config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("No providers available"));
        }
    }

    #[tokio::test]
    async fn test_fallback_multiple_providers() {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                enabled: true,
                model: "gpt-4".to_string(),
                temperature: 0.7,
                max_tokens: 2000,
                api_key: Some("test-key-1".to_string()),
                base_url: None,
                endpoint: None,
                deployment_name: None,
                api_version: None,
                project_id: None,
            },
        );
        providers.insert(
            "anthropic".to_string(),
            ProviderConfig {
                enabled: true,
                model: "claude-3-5-sonnet-20250929".to_string(),
                temperature: 0.7,
                max_tokens: 4000,
                api_key: Some("test-key-2".to_string()),
                base_url: None,
                endpoint: None,
                deployment_name: None,
                api_version: None,
                project_id: None,
            },
        );

        let config = AiConfig {
            default_provider: "openai".to_string(),
            providers,
            fallback: FallbackConfig {
                enabled: true,
                order: vec!["openai".to_string(), "anthropic".to_string()],
                retry_attempts: 2,
                retry_delay_ms: 50,
            },
        };

        let fallback = FallbackProvider::new(&config).unwrap();
        assert_eq!(fallback.providers.len(), 2);
    }
}

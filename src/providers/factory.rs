use crate::config::{AiConfig, ProviderConfig};
use crate::providers::{
    AnthropicProvider, AzureOpenAIProvider, GoogleProvider, LlmProvider, OllamaProvider,
    OpenAIProvider,
};
use std::error::Error;

pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider instance from configuration
    pub fn create(
        provider_name: &str,
        config: &ProviderConfig,
    ) -> Result<Box<dyn LlmProvider>, Box<dyn Error>> {
        // Validate that provider is enabled
        if !config.enabled {
            return Err(format!(
                "Provider '{}' is not enabled in configuration",
                provider_name
            )
            .into());
        }

        match provider_name {
            "openai" => Ok(Box::new(OpenAIProvider::new(config)?)),
            "anthropic" => Ok(Box::new(AnthropicProvider::new(config)?)),
            "azure_openai" => Ok(Box::new(AzureOpenAIProvider::new(config)?)),
            "google" => Ok(Box::new(GoogleProvider::new(config)?)),
            "ollama" => Ok(Box::new(OllamaProvider::new(config)?)),
            _ => Err(format!("Unknown provider: {}", provider_name).into()),
        }
    }

    /// Get the default provider from configuration
    pub fn get_default_provider(config: &AiConfig) -> Result<Box<dyn LlmProvider>, Box<dyn Error>> {
        let provider_name = &config.default_provider;
        let provider_config = config.providers.get(provider_name).ok_or_else(|| {
            format!(
                "Default provider '{}' not found in configuration",
                provider_name
            )
        })?;

        Self::create(provider_name, provider_config)
    }

    /// List all available provider names
    pub fn available_providers() -> Vec<&'static str> {
        vec!["openai", "anthropic", "azure_openai", "google", "ollama"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_provider_config() -> ProviderConfig {
        ProviderConfig {
            enabled: true,
            model: "test-model".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: Some("test-key".to_string()),
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        }
    }

    #[test]
    fn test_create_openai_provider() {
        let config = create_test_provider_config();
        let provider = ProviderFactory::create("openai", &config).unwrap();
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_create_anthropic_provider() {
        let config = create_test_provider_config();
        let provider = ProviderFactory::create("anthropic", &config).unwrap();
        assert_eq!(provider.provider_name(), "anthropic");
    }

    #[test]
    fn test_create_google_provider() {
        let config = create_test_provider_config();
        let provider = ProviderFactory::create("google", &config).unwrap();
        assert_eq!(provider.provider_name(), "google");
    }

    #[test]
    fn test_create_azure_provider() {
        let mut config = create_test_provider_config();
        config.endpoint = Some("https://test.openai.azure.com".to_string());
        config.deployment_name = Some("gpt-4".to_string());

        let provider = ProviderFactory::create("azure_openai", &config).unwrap();
        assert_eq!(provider.provider_name(), "azure_openai");
    }

    #[test]
    fn test_create_unknown_provider() {
        let config = create_test_provider_config();
        let result = ProviderFactory::create("unknown", &config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Unknown provider"));
        }
    }

    #[test]
    fn test_create_disabled_provider() {
        let mut config = create_test_provider_config();
        config.enabled = false;

        let result = ProviderFactory::create("openai", &config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not enabled in configuration"));
        }
    }

    #[test]
    fn test_get_default_provider() {
        let mut providers = HashMap::new();
        providers.insert("openai".to_string(), create_test_provider_config());

        let ai_config = AiConfig {
            default_provider: "openai".to_string(),
            providers,
            fallback: Default::default(),
        };

        let provider = ProviderFactory::get_default_provider(&ai_config).unwrap();
        assert_eq!(provider.provider_name(), "openai");
    }

    #[test]
    fn test_get_default_provider_not_found() {
        let providers = HashMap::new();

        let ai_config = AiConfig {
            default_provider: "openai".to_string(),
            providers,
            fallback: Default::default(),
        };

        let result = ProviderFactory::get_default_provider(&ai_config);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not found"));
        }
    }

    #[test]
    fn test_create_ollama_provider() {
        let config = create_test_provider_config();
        let provider = ProviderFactory::create("ollama", &config).unwrap();
        assert_eq!(provider.provider_name(), "ollama");
    }

    #[test]
    fn test_available_providers() {
        let providers = ProviderFactory::available_providers();
        assert_eq!(providers.len(), 5);
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"anthropic"));
        assert!(providers.contains(&"azure_openai"));
        assert!(providers.contains(&"google"));
        assert!(providers.contains(&"ollama"));
    }
}

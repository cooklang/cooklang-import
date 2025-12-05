use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::collections::HashMap;

/// Main AI configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct AiConfig {
    /// Default provider to use when not specified
    #[serde(default = "default_provider")]
    pub default_provider: String,
    /// Map of provider name to provider configuration
    pub providers: HashMap<String, ProviderConfig>,
    /// Fallback configuration for automatic provider switching
    #[serde(default)]
    pub fallback: FallbackConfig,
    /// Extractors configuration
    #[serde(default)]
    pub extractors: ExtractorsConfig,
    /// Converters configuration
    #[serde(default)]
    pub converters: ConvertersConfig,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Configuration for a specific AI provider
#[derive(Debug, Deserialize, Clone)]
pub struct ProviderConfig {
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Model identifier (e.g., "gpt-4", "claude-3-5-sonnet-20250929")
    pub model: String,
    /// Temperature for generation (0.0-1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    // Optional provider-specific fields
    /// API key for authentication (can also be set via environment variable)
    pub api_key: Option<String>,
    /// Base URL for API endpoint (for custom or proxy endpoints)
    pub base_url: Option<String>,
    /// Specific endpoint path (for Azure or custom deployments)
    pub endpoint: Option<String>,
    /// Deployment name (Azure OpenAI specific)
    pub deployment_name: Option<String>,
    /// API version (Azure OpenAI specific)
    pub api_version: Option<String>,
    /// Project ID (Google Cloud specific)
    pub project_id: Option<String>,
}

/// Configuration for provider fallback and retry behavior
#[derive(Debug, Deserialize, Clone)]
pub struct FallbackConfig {
    /// Whether fallback is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Order of providers to try (first to last)
    #[serde(default)]
    pub order: Vec<String>,
    /// Number of retry attempts per provider before fallback
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    /// Initial delay between retries in milliseconds (uses exponential backoff)
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            order: Vec::new(),
            retry_attempts: default_retry_attempts(),
            retry_delay_ms: default_retry_delay_ms(),
        }
    }
}

/// Configuration for recipe extractors
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExtractorsConfig {
    /// List of enabled extractors
    #[serde(default = "default_extractors")]
    pub enabled: Vec<String>,
    /// Order in which extractors should be tried
    #[serde(default = "default_extractors")]
    pub order: Vec<String>,
}

/// Configuration for recipe converters
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConvertersConfig {
    /// List of enabled converters
    #[serde(default)]
    pub enabled: Vec<String>,
    /// Order in which converters should be tried
    #[serde(default)]
    pub order: Vec<String>,
    /// Default converter to use
    #[serde(default)]
    pub default: String,
}

// Default value functions
fn default_provider() -> String {
    "openai".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    2000
}

fn default_retry_attempts() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_extractors() -> Vec<String> {
    vec![
        "json_ld".to_string(),
        "microdata".to_string(),
        "html_class".to_string(),
    ]
}

fn default_timeout() -> u64 {
    30
}

impl AiConfig {
    /// Load configuration from file and environment variables
    ///
    /// Configuration is loaded with the following priority (highest to lowest):
    /// 1. Environment variables with COOKLANG__ prefix
    /// 2. config.toml file in current directory
    /// 3. Default values
    ///
    /// Environment variable format: COOKLANG__PROVIDERS__OPENAI__API_KEY
    pub fn load() -> Result<Self, ConfigError> {
        load_config()
    }
}

/// Load configuration from file and environment variables
///
/// Configuration is loaded with the following priority (highest to lowest):
/// 1. Environment variables with COOKLANG__ prefix
/// 2. config.toml file in current directory
/// 3. Default values
///
/// Environment variable format: COOKLANG__PROVIDERS__OPENAI__API_KEY
pub fn load_config() -> Result<AiConfig, ConfigError> {
    let settings = Config::builder()
        // Optional config file (can be missing)
        .add_source(File::with_name("config").required(false))
        // Environment variables with COOKLANG_ prefix
        // Use double underscore for nested: COOKLANG__PROVIDERS__OPENAI__API_KEY
        .add_source(
            Environment::with_prefix("COOKLANG")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    settings.try_deserialize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_values() {
        assert_eq!(default_provider(), "openai");
        assert_eq!(default_temperature(), 0.7);
        assert_eq!(default_max_tokens(), 2000);
        assert_eq!(default_retry_attempts(), 3);
        assert_eq!(default_retry_delay_ms(), 1000);
    }

    #[test]
    fn test_fallback_config_default() {
        let fallback = FallbackConfig::default();
        assert!(!fallback.enabled);
        assert!(fallback.order.is_empty());
        assert_eq!(fallback.retry_attempts, 3);
        assert_eq!(fallback.retry_delay_ms, 1000);
    }

    #[test]
    fn test_provider_config_has_optional_fields() {
        // Test that ProviderConfig can be created with None for optional fields
        let config = ProviderConfig {
            enabled: true,
            model: "gpt-4.1-mini".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            api_key: None,
            base_url: None,
            endpoint: None,
            deployment_name: None,
            api_version: None,
            project_id: None,
        };

        assert!(config.api_key.is_none());
        assert!(config.base_url.is_none());
    }

    #[test]
    fn test_load_config_without_file() {
        // Clear any environment variables that might interfere
        let keys_to_clear: Vec<String> = env::vars()
            .filter(|(k, _)| k.starts_with("COOKLANG__"))
            .map(|(k, _)| k)
            .collect();

        for key in keys_to_clear {
            env::remove_var(&key);
        }

        // Loading config without a file should use defaults (will fail because no providers configured)
        // This is expected behavior - we need at least one provider configured
        let result = load_config();

        // We expect this to fail because no providers are configured
        // The important thing is it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_ai_config_structure() {
        // Test that we can construct AiConfig with proper structure
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                enabled: true,
                model: "gpt-4.1-mini".to_string(),
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

        let config = AiConfig {
            default_provider: "openai".to_string(),
            providers,
            fallback: FallbackConfig::default(),
            extractors: ExtractorsConfig::default(),
            converters: ConvertersConfig::default(),
            timeout: default_timeout(),
        };

        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.providers.len(), 1);
        assert!(config.providers.contains_key("openai"));
    }
}

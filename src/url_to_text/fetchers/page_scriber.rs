use super::Fetcher;

use super::domain_in_list;
use crate::config::{load_config, PageScriberConfig};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct SourceRequest {
    url: String,
}

#[derive(Deserialize)]
struct SourceResponse {
    source: String,
}

pub struct PageScriberFetcher {
    client: Client,
    config: PageScriberConfig,
}

impl PageScriberFetcher {
    pub fn new(config: PageScriberConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    pub fn from_env() -> Self {
        let config = load_config()
            .ok()
            .map(|c| c.page_scriber)
            .unwrap_or_default();
        Self::new(config)
    }

    pub(crate) fn empty() -> Self {
        Self::new(PageScriberConfig::default())
    }
}

#[async_trait]
impl Fetcher for PageScriberFetcher {
    /// Fetch HTML source from a URL via the page scriber service.
    /// Returns raw HTML that can be parsed by structured extractors.
    async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let endpoint = match &self.config.url {
            Some(ep) => ep.to_string(),
            None => return Err("Pagescriber not configured".into()),
        };
        let response = self
            .client
            .post(endpoint)
            .json(&SourceRequest {
                url: url.to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!(
                "Page scriber fetch failed with status: {}",
                response.status()
            )
            .into());
        }

        let resp: SourceResponse = response.json().await?;
        Ok(resp.source)
    }

    fn is_available(&self) -> bool {
        self.config.url.is_some()
    }

    fn is_configured(&self, url: &str) -> bool {
        domain_in_list(url, &self.config.domains)
    }

    fn name(&self) -> &str {
        "page_scriber"
    }

    fn fallback(&self) -> bool {
        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_without_url() {
        let fetcher = PageScriberFetcher::empty();
        assert!(!fetcher.is_available());
    }

    #[test]
    fn test_is_available_with_url() {
        let fetcher = PageScriberFetcher::new(PageScriberConfig {
            url: Some("http://localhost:4000".to_string()),
            domains: vec![],
        });
        assert!(fetcher.is_available());
    }

    fn match_configured(url: &str, domains: &Vec<&str>) -> bool {
        let fetcher = PageScriberFetcher::new(PageScriberConfig {
            url: Some("http://localhost:4000".to_string()),
            domains: domains.iter().map(|s| s.to_string()).collect(),
        });

        fetcher.is_configured(url)
    }
    #[test]
    fn test_domain_matches_exact() {
        let domains = vec!["seriouseats.com"];
        assert!(match_configured("https://seriouseats.com/recipe", &domains));
    }

    #[test]
    fn test_domain_matches_subdomain() {
        let domains = vec!["seriouseats.com"];

        assert!(match_configured(
            "https://www.seriouseats.com/recipe",
            &domains
        ));
    }

    #[test]
    fn test_domain_no_match() {
        let domains = vec!["seriouseats.com"];
        assert!(!match_configured("https://example.com/recipe", &domains));
    }

    #[test]
    fn test_domain_empty_list() {
        let domains = vec![];
        assert!(!match_configured(
            "https://seriouseats.com/recipe",
            &domains
        ));
    }

    #[test]
    fn test_domain_invalid_url() {
        let domains = vec!["seriouseats.com"];
        assert!(!match_configured("not-a-url", &domains));
    }
}

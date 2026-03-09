use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Serialize)]
struct SourceRequest {
    url: String,
}

#[derive(Deserialize)]
struct SourceResponse {
    content: String,
}

pub struct PageScriberFetcher {
    endpoint: String,
    client: Client,
}

impl PageScriberFetcher {
    pub fn new(page_scriber_url: Option<String>) -> Option<Self> {
        let base_url = page_scriber_url?;
        let endpoint = format!("{}/api/fetch-source", base_url);
        let client = Client::new();
        Some(Self { endpoint, client })
    }

    pub fn is_available(page_scriber_url: Option<&String>) -> bool {
        page_scriber_url.is_some()
    }

    /// Fetch HTML source from a URL via the page scriber service.
    /// Returns raw HTML that can be parsed by structured extractors.
    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .post(&self.endpoint)
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

        let content: SourceResponse = response.json().await?;
        Ok(content.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_none_without_url() {
        let fetcher = PageScriberFetcher::new(None);
        assert!(fetcher.is_none());
    }

    #[test]
    fn test_new_returns_some_with_url() {
        let fetcher = PageScriberFetcher::new(Some("http://localhost:4000".to_string()));
        assert!(fetcher.is_some());
    }

    #[test]
    fn test_is_available_without_url() {
        assert!(!PageScriberFetcher::is_available(None));
    }

    #[test]
    fn test_is_available_with_url() {
        assert!(PageScriberFetcher::is_available(Some(&"http://localhost:4000".to_string())));
    }
}

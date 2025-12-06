use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

#[derive(Serialize)]
struct ContentRequest {
    url: String,
}

#[derive(Deserialize)]
struct ContentResponse {
    content: String,
}

pub struct ChromeFetcher {
    endpoint: String,
    client: Client,
}

impl ChromeFetcher {
    pub fn new() -> Option<Self> {
        let page_scriber_url = env::var("PAGE_SCRIBER_URL").ok()?;
        let endpoint = format!("{}/api/fetch-content", page_scriber_url);
        let client = Client::new();
        Some(Self { endpoint, client })
    }

    pub fn is_available() -> bool {
        env::var("PAGE_SCRIBER_URL").is_ok()
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .post(&self.endpoint)
            .json(&ContentRequest {
                url: url.to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Chrome fetch failed with status: {}", response.status()).into());
        }

        let content: ContentResponse = response.json().await?;
        Ok(content.content)
    }
}

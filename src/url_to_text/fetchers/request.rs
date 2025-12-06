use reqwest::Client;
use std::error::Error;
use std::time::Duration;

pub struct RequestFetcher {
    client: Client,
}

impl RequestFetcher {
    pub fn new(timeout: Option<Duration>) -> Self {
        let timeout = timeout.unwrap_or(Duration::from_secs(30));
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("Mozilla/5.0 (compatible; CooklangBot/1.0)")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self.client.get(url).send().await?;
        let html = response.text().await?;
        Ok(html)
    }
}

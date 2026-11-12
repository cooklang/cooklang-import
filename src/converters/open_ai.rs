use super::openai_chat::{is_prompt_flagged, Generation, DEFAULT_MODEL, OPENAI_BASE_URL};
use super::{prompt_for_model, ConversionMetadata, ConversionResult, Converter, TokenUsage};
use crate::config::ProviderConfig;
use async_trait::async_trait;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Instant;

pub struct OpenAiConverter {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    reasoning_effort: Option<String>,
}

/// `OPENAI_REASONING_EFFORT`, when set to something.
fn reasoning_effort_from_env() -> Option<String> {
    std::env::var("OPENAI_REASONING_EFFORT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl OpenAiConverter {
    /// Create a new OpenAI converter from configuration
    pub fn new(config: &ProviderConfig) -> Result<Self, Box<dyn Error>> {
        // Try config first, then fall back to environment variable
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or("OPENAI_API_KEY not found in config or environment")?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| OPENAI_BASE_URL.to_string());

        Ok(OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            reasoning_effort: reasoning_effort_from_env()
                .or_else(|| config.reasoning_effort.clone()),
        })
    }

    /// Create a new OpenAI converter from environment variables
    ///
    /// Uses OPENAI_API_KEY, OPENAI_MODEL (defaults to gpt-6-luna) and
    /// OPENAI_REASONING_EFFORT (defaults to none) from environment
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| "OPENAI_API_KEY environment variable not set")?;

        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Ok(OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url: OPENAI_BASE_URL.to_string(),
            model,
            temperature: 0.2,
            max_tokens: 4000,
            reasoning_effort: reasoning_effort_from_env(),
        })
    }

    #[doc(hidden)]
    pub fn with_base_url(api_key: String, base_url: String, model: String) -> Self {
        OpenAiConverter {
            client: Client::new(),
            api_key,
            base_url,
            model,
            temperature: 0.2,
            max_tokens: 2000,
            reasoning_effort: None,
        }
    }

    fn request_body(&self, content: &str) -> Value {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": prompt_for_model(&self.model, content)}
            ],
            "stream": false
        });
        Generation {
            model: &self.model,
            openai_host: self.base_url == OPENAI_BASE_URL,
            reasoning_effort: self.reasoning_effort.as_deref(),
            temperature: Some(self.temperature),
            max_output_tokens: Some(self.max_tokens),
        }
        .apply(&mut body);
        body
    }

    /// One request. Returns the status-independent parsed body.
    async fn send(&self, body: &Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept-Encoding", "identity")
            .json(body)
            .send()
            .await?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body (status {}): {}", status, e))?;
        debug!("Raw response: {}", response_text);

        serde_json::from_str(&response_text).map_err(|e| {
            format!(
                "Failed to parse JSON: {}. Raw response: {}",
                e,
                crate::pipelines::truncate_on_char_boundary(response_text.clone(), 500)
            )
            .into()
        })
    }
}

#[async_trait]
impl Converter for OpenAiConverter {
    fn name(&self) -> &str {
        "open_ai"
    }

    async fn convert(
        &self,
        content: &str,
    ) -> Result<ConversionResult, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();
        let body = self.request_body(content);

        let mut response_body = self.send(&body).await?;
        // The input filter refuses the odd ordinary recipe and lets the same
        // request through on a second try.
        if response_body["error"]["message"]
            .as_str()
            .is_some_and(is_prompt_flagged)
        {
            debug!("Prompt flagged by the input filter; retrying once");
            response_body = self.send(&body).await?;
        }

        let latency_ms = start.elapsed().as_millis() as u64;

        // Check for API error response
        if let Some(error) = response_body.get("error") {
            let error_msg = error["message"].as_str().unwrap_or("Unknown API error");
            return Err(format!("OpenAI API error: {}", error_msg).into());
        }

        // Cut off at the token cap: half a recipe must not be stored as a recipe.
        if response_body["choices"][0]["finish_reason"].as_str() == Some("length") {
            return Err("Response truncated: the recipe exceeded the model's output limit".into());
        }

        let cooklang_recipe = response_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                format!(
                    "Failed to extract content from response. Response: {}",
                    serde_json::to_string_pretty(&response_body)
                        .unwrap_or_else(|_| "unparseable".to_string())
                )
            })?
            .to_string();

        // Extract metadata from response
        let model_version = response_body["model"].as_str().map(|s| s.to_string());
        let input_tokens = response_body["usage"]["prompt_tokens"]
            .as_u64()
            .map(|v| v as u32);
        let output_tokens = response_body["usage"]["completion_tokens"]
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
    use mockito::Server;

    #[tokio::test]
    async fn test_convert() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "choices": [{
                        "message": {
                            "content": ">> ingredients\n@pasta{500%g}\n@sauce\n\n>> instructions\n1. Cook pasta\n2. Add sauce"
                        }
                    }]
                }"#,
            )
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "pasta\nsauce\n\nCook pasta with sauce";

        let result = converter.convert(content).await.unwrap();
        assert!(result.content.contains("@pasta"));
        assert!(result.content.contains("@sauce"));
        mock.assert();
    }

    #[tokio::test]
    async fn test_convert_api_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid request"}"#)
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );
        let content = "ingredient\n\nstep";

        let result = converter.convert(content).await;
        assert!(result.is_err());
        mock.assert();
    }

    // Regression: the "Raw response" excerpt in the parse-failure message sliced
    // the body at a fixed byte offset. Any non-JSON response longer than the excerpt
    // whose 500th byte fell inside a multi-byte character panicked the worker thread
    // while building the error message for a failure it was about to report anyway.
    #[tokio::test]
    async fn test_convert_reports_unparseable_body_without_panicking() {
        let mut server = Server::new_async().await;
        // 499 ASCII bytes, then a 2-byte character straddling index 500.
        let body = format!("{}{}", "a".repeat(499), "é".repeat(20));
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(&body)
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-3.5-turbo".to_string(),
        );

        let err = converter
            .convert("ingredient\n\nstep")
            .await
            .expect_err("a non-JSON body must be reported as an error");
        assert!(
            err.to_string().contains("Failed to parse JSON"),
            "got: {err}"
        );
        mock.assert();
    }

    // gpt-6-luna answers 400 to `max_tokens`, and to a non-default temperature
    // unless reasoning is off.
    #[test]
    fn test_request_body_for_the_default_model() {
        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            OPENAI_BASE_URL.to_string(),
            DEFAULT_MODEL.to_string(),
        );
        let body = converter.request_body("2 eggs\n\nBoil.");
        assert_eq!(body["model"], "gpt-6-luna");
        assert_eq!(body["reasoning_effort"], "none");
        assert_eq!(body["max_completion_tokens"], 2000);
        assert!(body.get("max_tokens").is_none());
        assert!(body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("2 eggs"));
    }

    #[tokio::test]
    async fn test_truncated_answer_is_an_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"choices": [{"finish_reason": "length",
                    "message": {"content": "Whisk the @flour{200%g} and"}}]}"#,
            )
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-6-luna".to_string(),
        );
        let err = converter
            .convert("flour\n\nWhisk.")
            .await
            .expect_err("a truncated answer must not be returned as a recipe");
        assert!(err.to_string().contains("truncated"), "got: {err}");
        mock.assert();
    }

    #[tokio::test]
    async fn test_flagged_prompt_is_retried_once() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"error": {"message": "Invalid prompt: your prompt was flagged as potentially violating our usage policy."}}"#,
            )
            .expect(2)
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-6-luna".to_string(),
        );
        let err = converter.convert("flour\n\nWhisk.").await.unwrap_err();
        assert!(err.to_string().contains("flagged"), "got: {err}");
        mock.assert();
    }

    #[tokio::test]
    async fn test_other_api_errors_are_not_retried() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(429)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": {"message": "Rate limit reached"}}"#)
            .expect(1)
            .create();

        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            server.url(),
            "gpt-6-luna".to_string(),
        );
        assert!(converter.convert("flour\n\nWhisk.").await.is_err());
        mock.assert();
    }

    #[tokio::test]
    async fn test_converter_name() {
        let converter = OpenAiConverter::with_base_url(
            "fake_api_key".to_string(),
            "https://api.openai.com".to_string(),
            "gpt-4.1-mini".to_string(),
        );
        assert_eq!(converter.name(), "open_ai");
    }
}

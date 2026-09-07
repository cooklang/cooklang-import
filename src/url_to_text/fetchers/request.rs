use super::Fetcher;
use async_trait::async_trait;
use reqwest::Client;
use std::error::Error;
use std::time::Duration;

/// Refuse response bodies larger than this before reading them.
///
/// Real recipe pages sampled in production topped out around 2.3 MB of HTML. 20 MB
/// leaves an order of magnitude of headroom while still ruling out media files,
/// which is what this guard is for.
pub(crate) const MAX_RESPONSE_BYTES: u64 = 20 * 1024 * 1024;

/// Content types that are definitely not a web page.
///
/// Deliberately a blocklist rather than an allowlist: sites serve HTML under all
/// sorts of odd content types, and refusing an unfamiliar-but-textual response
/// would break imports that work today. Everything here is unambiguously binary.
const BINARY_TYPES: [&str; 8] = [
    "image/",
    "video/",
    "audio/",
    "font/",
    "application/pdf",
    "application/zip",
    "application/octet-stream",
    "application/x-",
];

/// Whether a Content-Type could plausibly be a web page. A missing header is
/// treated as fetchable - absence of a declaration is not evidence of binary.
pub(crate) fn is_textual(content_type: Option<&str>) -> bool {
    let Some(ct) = content_type else {
        return true;
    };
    let ct = ct.trim().to_ascii_lowercase();
    !BINARY_TYPES.iter().any(|b| ct.starts_with(b))
}

/// Whether a declared Content-Length is within [`MAX_RESPONSE_BYTES`]. A missing
/// length is allowed: chunked responses do not declare one.
pub(crate) fn size_is_sane(content_length: Option<u64>) -> bool {
    content_length.is_none_or(|len| len <= MAX_RESPONSE_BYTES)
}

pub struct RequestFetcher {
    client: Client,
}

impl Default for RequestFetcher {
    fn default() -> Self {
        Self::new(None)
    }
}

impl RequestFetcher {
    pub fn new(timeout: Option<Duration>) -> Self {
        let timeout = timeout.unwrap_or(Duration::from_secs(30));
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

#[async_trait]
impl Fetcher for RequestFetcher {
    async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Failed to fetch page: HTTP {} ({})",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )
            .into());
        }

        // Reject media before reading the body. Without this a PDF or MP4 is decoded
        // as lossy text, parsed as HTML and handed to the LLM: a recipe PDF on S3
        // failed as "No recipe found in the text" and an MP4 was large enough to trip
        // OpenAI's request-size limit (production, 2026-08-17..24).
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        if !is_textual(content_type.as_deref()) {
            return Err(format!(
                "Not a web page: the URL returned {}",
                content_type.as_deref().unwrap_or("an unknown content type")
            )
            .into());
        }

        if !size_is_sane(response.content_length()) {
            return Err(format!(
                "Response too large: {} bytes exceeds the {} byte limit",
                response.content_length().unwrap_or(0),
                MAX_RESPONSE_BYTES
            )
            .into());
        }

        let html = response.text().await?;
        Ok(html)
    }

    fn is_available(&self) -> bool {
        true
    }
    fn is_configured(&self, _url: &str) -> bool {
        false
    }
    fn name(&self) -> &str {
        "request"
    }
    fn fallback(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_content_type_is_accepted() {
        for ct in [
            "text/html",
            "text/html; charset=utf-8",
            "text/html;charset=windows-1251",
            "application/xhtml+xml",
            "text/plain",
            "TEXT/HTML",
        ] {
            assert!(is_textual(Some(ct)), "{ct} should be fetchable");
        }
    }

    #[test]
    fn test_missing_content_type_is_accepted() {
        // Be permissive: a server that declares nothing is not evidence of binary.
        assert!(is_textual(None));
    }

    #[test]
    fn test_binary_content_types_are_rejected() {
        // Both observed in production on 2026-08-17..24: a recipe PDF on S3 came back
        // as mojibake and failed with "No recipe found in the text", and an MP4 from
        // jwpsrv.com was large enough to trip the OpenAI request-size limit.
        for ct in [
            "application/pdf",
            "image/jpeg",
            "image/png",
            "video/mp4",
            "audio/mpeg",
            "font/woff2",
            "application/zip",
            "application/octet-stream",
            "APPLICATION/PDF",
        ] {
            assert!(!is_textual(Some(ct)), "{ct} should be rejected");
        }
    }

    #[test]
    fn test_size_limit_rejects_oversized_bodies() {
        assert!(!size_is_sane(Some(MAX_RESPONSE_BYTES + 1)));
        assert!(size_is_sane(Some(MAX_RESPONSE_BYTES)));
        assert!(size_is_sane(Some(1_000)));
        // No Content-Length is common with chunked transfer encoding; allow it.
        assert!(size_is_sane(None));
    }

    #[tokio::test]
    async fn test_fetch_rejects_pdf_with_a_clear_message() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/recipe.pdf")
            .with_status(200)
            .with_header("content-type", "application/pdf")
            .with_body(&b"%PDF-1.4 binary junk"[..])
            .create_async()
            .await;

        let err = RequestFetcher::new(None)
            .fetch(&format!("{}/recipe.pdf", server.url()))
            .await
            .expect_err("a PDF must not be fetched as HTML");
        let err = err.to_string();
        assert!(err.contains("application/pdf"), "got: {err}");
        assert!(err.to_lowercase().contains("not a web page"), "got: {err}");
    }

    #[tokio::test]
    async fn test_fetch_rejects_oversized_response() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/huge")
            .with_status(200)
            .with_header("content-type", "text/html")
            // A real oversized body: hyper rejects a Content-Length that does not
            // match what is actually sent.
            .with_body(vec![b'x'; (MAX_RESPONSE_BYTES + 1) as usize])
            .create_async()
            .await;

        let err = RequestFetcher::new(None)
            .fetch(&format!("{}/huge", server.url()))
            .await
            .expect_err("an oversized body must be refused before reading it");
        assert!(err.to_string().contains("too large"), "got: {err}");
    }

    #[tokio::test]
    async fn test_fetch_still_returns_normal_html() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/recipe")
            .with_status(200)
            .with_header("content-type", "text/html; charset=utf-8")
            .with_body("<html><body><h1>Soup</h1></body></html>")
            .create_async()
            .await;

        let html = RequestFetcher::new(None)
            .fetch(&format!("{}/recipe", server.url()))
            .await
            .unwrap();
        assert!(html.contains("Soup"));
    }
}

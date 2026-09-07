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

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// The headers a real Chrome navigation sends alongside its user agent.
///
/// A Chrome UA with none of these is a stock bot fingerprint, and WAFs answer it
/// with 403 (thekitchn.com, 2026-09-05: 403 to the bare UA, 200 and three JSON-LD
/// Recipe blocks with the full set). `Accept-Encoding` is deliberately absent —
/// reqwest sets it from its own decompression features and overriding it would make
/// it advertise encodings it cannot decode.
fn browser_headers() -> reqwest::header::HeaderMap {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    const HEADERS: [(&str, &str); 8] = [
        (
            "accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        ),
        ("accept-language", "en-US,en;q=0.9"),
        ("upgrade-insecure-requests", "1"),
        ("sec-fetch-dest", "document"),
        ("sec-fetch-mode", "navigate"),
        ("sec-fetch-site", "none"),
        ("sec-fetch-user", "?1"),
        ("sec-ch-ua-platform", "\"macOS\""),
    ];

    let mut headers = HeaderMap::new();
    for (name, value) in HEADERS {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
    headers
}

pub struct RequestFetcher {
    client: Client,
}

impl RequestFetcher {
    pub fn new(timeout: Option<Duration>) -> Self {
        let timeout = timeout.unwrap_or(Duration::from_secs(30));
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .default_headers(browser_headers())
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn fetch(&self, url: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
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

    // Regression: thekitchn.com, 2026-09-05.
    //
    // The client sent a Chrome user agent and nothing else. A "Chrome" that omits
    // Accept, Accept-Language and the Sec-Fetch-* set is a stock bot fingerprint, and
    // WAFs answer it with 403 — thekitchn.com returns 403 to the bare UA and 200 with
    // three valid JSON-LD Recipe blocks once the rest of a real navigation is present.
    #[tokio::test]
    async fn test_fetch_sends_a_complete_browser_header_set() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/recipe")
            .match_header("accept", mockito::Matcher::Regex("text/html".into()))
            .match_header("accept-language", mockito::Matcher::Any)
            .match_header("accept-encoding", mockito::Matcher::Regex("gzip".into()))
            .match_header("upgrade-insecure-requests", "1")
            .match_header("sec-fetch-dest", "document")
            .match_header("sec-fetch-mode", "navigate")
            .match_header("sec-fetch-site", "none")
            .match_header("sec-fetch-user", "?1")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("<html><body><h1>Salad</h1></body></html>")
            .create_async()
            .await;

        let html = RequestFetcher::new(None)
            .fetch(&format!("{}/recipe", server.url()))
            .await
            .expect("the request must carry a full browser header set");

        assert!(html.contains("Salad"));
        mock.assert_async().await;
    }

    // Regression: hostthetoast.com, 2026-09-06.
    //
    // Some CDNs (Sucuri here) answer with `content-encoding: gzip` even when the
    // request carries no `Accept-Encoding` header. Without reqwest's decompression
    // features the gzip stream was decoded as text, producing mojibake that parsed
    // into a garbage "page" — which then blew the LLM input cap and panicked the
    // worker thread.
    #[tokio::test]
    async fn test_fetch_decompresses_gzip_the_server_sent_unasked() {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        // Repetitive filler so deflate emits a real compressed block: on a short,
        // incompressible body it falls back to a *stored* block, and the marker
        // strings below would survive in the raw bytes whether or not the client
        // decompressed anything.
        let filler = "<p>stir the dough until it is smooth</p>".repeat(200);
        let page =
            format!("<html><body><h1>Garlic Naan</h1>{filler}<p>500 g flour</p></body></html>");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(page.as_bytes()).unwrap();
        let gzipped = encoder.finish().unwrap();
        assert!(
            !String::from_utf8_lossy(&gzipped).contains("Garlic Naan"),
            "body was stored uncompressed - the test would pass without decompression"
        );

        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/naan")
            .with_status(200)
            .with_header("content-type", "text/html; charset=UTF-8")
            .with_header("content-encoding", "gzip")
            .with_body(gzipped)
            .create_async()
            .await;

        let html = RequestFetcher::new(None)
            .fetch(&format!("{}/naan", server.url()))
            .await
            .expect("a gzipped page must be fetchable");

        assert!(html.contains("Garlic Naan"), "not decompressed: {html:?}");
        assert!(html.contains("500 g flour"), "not decompressed: {html:?}");
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

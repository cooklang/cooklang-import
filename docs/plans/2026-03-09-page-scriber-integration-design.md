# Page Scriber Integration with Domain-Aware Routing

## Problem

Some websites (e.g., Serious Eats, AllRecipes) block bot requests via Cloudflare, returning 402 or CAPTCHA pages. The current `RequestFetcher` (reqwest) fails on these sites, and the existing `ChromeFetcher` fallback returns plain text (via `/api/fetch-content`), bypassing structured extractors entirely and going straight to LLM extraction.

## Solution

Replace `ChromeFetcher` with a new `PageScriberFetcher` that hits `/api/fetch-source` to get **HTML source** instead of plain text. This allows structured extractors (JSON-LD, MicroData, HtmlClass) to work on browser-rendered HTML before falling back to LLM extraction.

Add a configurable domain list so known-blocked sites go straight to the page scriber, avoiding the wasted reqwest attempt.

## Config

New `page_scriber` section in `config.toml`:

```toml
[page_scriber]
url = "http://localhost:4000"
domains = ["seriouseats.com", "allrecipes.com"]
```

- `url`: Base URL of the page scriber service (replaces `PAGE_SCRIBER_URL` env var)
- `domains`: List of domains that should skip reqwest and use page scriber directly. Suffix-matched (e.g., `seriouseats.com` matches `www.seriouseats.com`)

## New Fetcher: `PageScriberFetcher`

File: `src/url_to_text/fetchers/page_scriber.rs`

- POST `{url}/api/fetch-source` with `{"url": "..."}`
- Returns raw HTML (not plain text)
- Same interface pattern as `RequestFetcher`

## Pipeline Flow

```
1. Extract domain from URL
2. Is domain in page_scriber.domains list?
   YES → PageScriberFetcher.fetch(url) → HTML
   NO  → RequestFetcher.fetch(url) → HTML or error

3. Got HTML → try structured extractors (JSON-LD → MicroData → HtmlClass)
   ├─ Success → return recipe
   └─ Fail → continue

4. If step 2 was RequestFetcher AND it failed (402/blocked):
   → PageScriberFetcher.fetch(url) → HTML → retry structured extractors
   ├─ Success → return recipe
   └─ Fail → continue

5. Final fallback: TextExtractor (LLM) on extracted text from whatever HTML we have
```

## Changes Summary

- **Remove**: `ChromeFetcher` (`src/url_to_text/fetchers/chrome.rs`) — fully replaced
- **Add**: `PageScriberFetcher` (`src/url_to_text/fetchers/page_scriber.rs`)
- **Modify**: `AiConfig` in `config.rs` — add `PageScriberConfig`
- **Modify**: `pipelines/url.rs` — domain-aware routing + auto-fallback
- **Remove**: `PAGE_SCRIBER_URL` env var dependency (moved to config)

## Error Handling

- Page scriber not configured (`url` is None) → auto-fallback skipped, same as today
- Page scriber unreachable → clear error: "Page scriber at {url} is not reachable"
- Malformed URL (can't extract domain) → falls through to reqwest path

mod page_scriber;
mod request;

pub use page_scriber::PageScriberFetcher;
pub use request::RequestFetcher;

/// Check if a URL's domain matches any domain in the list (suffix-matched).
/// "seriouseats.com" matches "www.seriouseats.com", "m.seriouseats.com", etc.
pub(crate) fn domain_in_list(url: &str, domains: &[String]) -> bool {
    let host = url
        .split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("");

    domains
        .iter()
        .any(|domain| host == domain.as_str() || host.ends_with(&format!(".{}", domain)))
}

#[async_trait::async_trait]
pub trait Fetcher {
    async fn fetch(&self, url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Whether the fetcher *can* be used, if false the fetcher will be ignored
    fn is_available(&self) -> bool;

    /// Whether the fetcher has been specifically configured this domain, this is used to create a priority sort.
    fn is_configured(&self, url: &str) -> bool;

    /// Whether the fetcher should be used as a fallback (even if the url does not match)
    fn fallback(&self) -> bool;

    fn name(&self) -> &str;
}

pub type DynFetcher = Box<dyn Fetcher>;

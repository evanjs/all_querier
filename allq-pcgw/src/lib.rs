use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::Context;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

pub const SUPPORTED_TYPES: &[&str] = &["video-game"];

/// Returns the canonical PCGW wiki page URL for a given page title/slug.
pub fn page_url(title: &str) -> String {
    format!("https://www.pcgamingwiki.com/wiki/{title}")
}

/// Returns the MediaWiki parse API URL for a given page title/slug.
pub fn parse_api_url(title: &str) -> String {
    format!(
        "https://www.pcgamingwiki.com/w/api.php\
         ?action=parse&redirects=1&page={}&format=json",
        urlencoding::encode(title),
    )
}

/// Extracts the `parse` object from a raw MediaWiki parse API JSON response body.
/// Returns an error if the response contains an API-level error or is missing the `parse` field.
pub fn parse_page_response(body: &str) -> anyhow::Result<Value> {
    let root: Value = serde_json::from_str(body)
        .context("failed to parse PCGW API response as JSON")?;
    if let Some(err) = root.get("error") {
        anyhow::bail!("PCGW API error: {err}");
    }
    root.get("parse")
        .cloned()
        .context("PCGW API response missing 'parse' field")
}

/// Rate limit: 20 requests/minute → 3 seconds between requests.
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(3000);

/// A `SearchProvider` backed by the PCGamingWiki MediaWiki search API.
pub struct PcgwSearchProvider {
    client: reqwest::Client,
    cache: Option<ProviderCache>,
}

impl PcgwSearchProvider {
    pub fn new(user_agent: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .expect("failed to build PCGW HTTP client");
        Self { client, cache: None }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(user_agent: &str, cache: ProviderCache) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .expect("failed to build PCGW HTTP client");
        Self { client, cache: Some(cache) }
    }

    /// Look up a cached JSON string for `key`, respecting `fetch_mode`.
    async fn cache_get(&self, key: &str, fetch_mode: FetchMode) -> Option<String> {
        if fetch_mode == FetchMode::ForceFetch {
            return None;
        }
        let cache = self.cache.as_ref()?;
        cache.get(key).await.ok().flatten().map(|e| e.value().clone())
    }

    /// Insert `value` into the cache under `key` unless in `CacheOnly` mode.
    async fn cache_insert(&self, key: String, value: String, fetch_mode: FetchMode) {
        if fetch_mode == FetchMode::CacheOnly {
            return;
        }
        if let Some(cache) = &self.cache {
            cache.insert(key, value);
        }
    }

    /// Fetch full page data via the `parse` action, following redirects.
    /// Checks the cache first (keyed by title), stores the result on a miss.
    async fn parse_page(&self, title: &str, fetch_mode: FetchMode) -> anyhow::Result<Value> {
        let cache_key = format!("pcgw:parse:{title}");

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            return serde_json::from_str(&cached)
                .context("failed to deserialize cached PCGW parse response");
        }

        if fetch_mode == FetchMode::CacheOnly {
            anyhow::bail!("PCGW parse for '{title}' not in cache (cache-only mode)");
        }

        let url = parse_api_url(title);
        debug!(%url, "parsing PCGW page");

        let body = self
            .client
            .get(&url)
            .send()
            .await
            .context("PCGW parse request failed")?
            .error_for_status()
            .context("PCGW parse returned error status")?
            .text()
            .await
            .context("failed to read PCGW parse response body")?;

        let value = parse_page_response(&body)?;

        if let Ok(json) = serde_json::to_string(&value) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }

        Ok(value)
    }
}

#[async_trait]
impl SearchProvider for PcgwSearchProvider {
    fn name(&self) -> &'static str {
        "pcgw"
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        // PCGW only indexes video games — skip if the caller asked for a different type.
        if let Some(t) = item_type {
            if t != "video-game" {
                return Ok(Vec::new());
            }
        }

        let fetch_mode = options.fetch_mode;
        let limit = options.limit.unwrap_or(10).min(50);
        let search_cache_key = format!("pcgw:search:{query}:{limit}");

        // Try to serve the full result list from cache.
        if let Some(cached) = self.cache_get(&search_cache_key, fetch_mode).await {
            let value: serde_json::Value = serde_json::from_str(&cached)
                .context("failed to deserialize cached PCGW search results")?;
            let results: Vec<SearchResult> = serde_json::from_value(value)
                .context("failed to convert cached PCGW search results")?;
            return Ok(results);
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let url = format!(
            "https://www.pcgamingwiki.com/w/api.php\
             ?action=query&list=search&srsearch={}&srlimit={}&format=json",
            urlencoding::encode(query),
            limit,
        );

        debug!(%url, "searching PCGW");

        let body: Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("PCGW search request failed")?
            .error_for_status()
            .context("PCGW search returned error status")?
            .json()
            .await
            .context("failed to parse PCGW search response")?;

        let results = body
            .pointer("/query/search")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut out = Vec::with_capacity(results.len());

        for entry in results {
            let title = match entry.get("title").and_then(Value::as_str) {
                Some(t) => t.to_string(),
                None => continue,
            };
            let pageid = match entry.get("pageid").and_then(Value::as_u64) {
                Some(id) => id,
                None => continue,
            };

            // Respect the 20 req/min rate limit before each parse call.
            tokio::time::sleep(RATE_LIMIT_DELAY).await;

            let parse_data = match self.parse_page(&title, fetch_mode).await {
                Ok(v) => v,
                Err(e) => {
                    debug!(error = %e, title, "PCGW parse failed, falling back to search entry");
                    entry.clone()
                }
            };

            // Prefer the resolved title/pageid from the parse response.
            let resolved_title = parse_data
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&title)
                .to_string();

            let resolved_id = parse_data
                .get("pageid")
                .and_then(Value::as_u64)
                .unwrap_or(pageid)
                .to_string();

            let description = entry
                .get("snippet")
                .and_then(Value::as_str)
                .map(|s| ammonia::clean_text(s));

            out.push(SearchResult {
                provider: "pcgw".to_string(),
                id: resolved_id,
                label: resolved_title,
                description,
                item_type: Some("video-game".to_string()),
                data: parse_data,
            });
        }

        // Cache the assembled result list.
        if let Ok(json) = serde_json::to_string(&out) {
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(out)
    }
}

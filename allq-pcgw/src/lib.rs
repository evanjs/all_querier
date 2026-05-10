use allq_core::{SearchOptions, SearchProvider, SearchResult};
use anyhow::Context;
use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tracing::debug;

pub const SUPPORTED_TYPES: &[&str] = &["video-game"];

/// Rate limit: 20 requests/minute → 3 seconds between requests.
const RATE_LIMIT_DELAY: Duration = Duration::from_millis(3000);

/// A `SearchProvider` backed by the PCGamingWiki MediaWiki search API.
pub struct PcgwSearchProvider {
    client: reqwest::Client,
}

impl PcgwSearchProvider {
    pub fn new(user_agent: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .expect("failed to build PCGW HTTP client");
        Self { client }
    }

    /// Fetch full page data via the `parse` action, following redirects.
    async fn parse_page(&self, title: &str) -> anyhow::Result<Value> {
        let url = format!(
            "https://www.pcgamingwiki.com/w/api.php\
             ?action=parse&redirects=1&page={}&format=json",
            urlencoding::encode(title),
        );

        debug!(%url, "parsing PCGW page");

        let body: Value = self
            .client
            .get(&url)
            .send()
            .await
            .context("PCGW parse request failed")?
            .error_for_status()
            .context("PCGW parse returned error status")?
            .json()
            .await
            .context("failed to parse PCGW parse response")?;

        Ok(body)
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

        let limit = options.limit.unwrap_or(10).min(50);

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

            let parse_data = match self.parse_page(&title).await {
                Ok(v) => v,
                Err(e) => {
                    debug!(error = %e, title, "PCGW parse failed, falling back to search entry");
                    entry.clone()
                }
            };

            // Prefer the resolved title/pageid from the parse response.
            let resolved_title = parse_data
                .pointer("/parse/title")
                .and_then(Value::as_str)
                .unwrap_or(&title)
                .to_string();

            let resolved_id = parse_data
                .pointer("/parse/pageid")
                .and_then(Value::as_u64)
                .unwrap_or(pageid)
                .to_string();

            let description = entry
                .get("snippet")
                .and_then(Value::as_str)
                .map(|s| ammonia::clean_text(s));

            out.push(SearchResult {
                provider: "pcgw",
                id: resolved_id,
                label: resolved_title,
                description,
                item_type: Some("video-game".to_string()),
                data: parse_data,
            });
        }

        Ok(out)
    }
}

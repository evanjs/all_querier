pub mod cache;
pub mod dispatcher;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use cache::{ProviderCache, all_querier_data_dir, create_provider_cache};
pub use dispatcher::SearchDispatcher;

/// Controls how a provider resolves requests relative to its local cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FetchMode {
    /// Use the cache when available; fall back to the network (default).
    #[default]
    NetworkFallback,
    /// Only read from the local cache; never call the network.
    CacheOnly,
    /// Always fetch from the network; ignore any cached data.
    ForceFetch,
}

/// Options that control search behavior (e.g. pagination, language).
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: Option<u32>,
    /// Language code for labels/descriptions (e.g. "en").
    pub language: Option<String>,
    /// How to resolve requests relative to the local cache.
    pub fetch_mode: FetchMode,
    /// Optional media sub-type filter (e.g. "tv", "ova", "movie", "manga", "novel").
    /// Currently applied post-search by the MAL provider; other providers ignore it.
    pub media_type: Option<String>,
    /// Optional MyAnimeList username for user-list endpoints.
    pub mal_username: Option<String>,
    /// Include NSFW results (passed to MAL `.nsfw()` parameter).
    pub nsfw: bool,
}

/// A provider-agnostic search result envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Which provider produced this result (e.g. "wikidata", "musicbrainz").
    pub provider: String,
    /// Provider-specific identifier (QID, MBID, MAL ID, etc.).
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Optional short description.
    pub description: Option<String>,
    /// Item type if known (e.g. "album", "character", "video-game").
    pub item_type: Option<String>,
    /// Full provider-specific payload.
    pub data: serde_json::Value,
}

/// Trait that all search backends implement.
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Human-readable name, e.g. "wikidata", "musicbrainz", "myanimelist".
    fn name(&self) -> &'static str;

    /// Which item types this provider can search for.
    fn supported_item_types(&self) -> &[&str];

    /// Search by free-text query, optionally constrained to an item type.
    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>>;
}

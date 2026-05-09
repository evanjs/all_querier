pub mod dispatcher;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use dispatcher::SearchDispatcher;

/// Options that control search behavior (e.g. pagination, language).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    pub limit: Option<u32>,
    /// Language code for labels/descriptions (e.g. "en").
    pub language: Option<String>,
}

/// A provider-agnostic search result envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Which provider produced this result (e.g. "wikidata", "musicbrainz").
    pub provider: &'static str,
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

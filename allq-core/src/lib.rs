pub mod cache;
pub mod dispatcher;

use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{VariantNames, EnumString};
use derive_more::FromStr;

pub use cache::{ProviderCache, all_querier_cache_dir, create_provider_cache};
pub use dispatcher::SearchDispatcher;

pub const REQWEST_VERSION: &str = env!("REQWEST_VERSION");

#[derive(Debug, Clone, VariantNames, EnumString)]
pub enum GameStoreType {
    Steam,
    Gog
}

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
    pub anilist_username: Option<String>,
    /// Include NSFW results (passed to MAL `.nsfw()` parameter).
    pub nsfw: bool,
    pub provider_direct_id_search: Option<GameStoreType>
}

/// Options that control search behavior (e.g. provider_id_direct_search).
#[derive(Debug, Clone, Default)]
pub struct GameSearchOptions {
    /// Maximum number of results to return.
    pub limit: Option<u32>,
    /// Language code for labels/descriptions (e.g. "en").
    pub language: Option<String>,
    /// How to resolve requests relative to the local cache.
    pub fetch_mode: FetchMode,
    /// Include NSFW results (passed to MAL `.nsfw()` parameter).
    // pub nsfw: bool,
    pub provider_direct_id_search: Option<GameStoreType>
}

impl From<SearchOptions> for GameSearchOptions {
    fn from(value: SearchOptions) -> Self {
        GameSearchOptions {
            limit: value.limit,
            language: value.language,
            fetch_mode: value.fetch_mode,
            provider_direct_id_search: value.provider_direct_id_search,
        }
    }
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

impl Default for SearchResult {
    fn default() -> Self {
        Self {
            provider: "".to_string(),
            id: "".to_string(),
            label: "".to_string(),
            description: None,
            item_type: None,
            data: Default::default(),
        }
    }
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

pub const GAME_SEARCH_SUPPORTED_TYPES: &[&str] = &["video-game"];

#[async_trait]
pub trait GameSearchProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn search_games(
        &self,
        query: &str,
        options: &GameSearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>>;
}

impl From<GameSearchOptions> for SearchOptions {
    fn from(value: GameSearchOptions) -> Self {
        Self {
            limit: value.limit,
            language: value.language,
            fetch_mode: value.fetch_mode,
            media_type: None,
            mal_username: None,
            anilist_username: None,
            nsfw: false,
            provider_direct_id_search: value.provider_direct_id_search,
        }
    }
}

#[async_trait]
impl<T> SearchProvider for T
where
    T: GameSearchProvider + ?Sized,
{
    fn name(&self) -> &'static str {
        GameSearchProvider::name(self)
    }

    fn supported_item_types(&self) -> &[&str] {
        GAME_SEARCH_SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        if item_type.is_some_and(|item_type| item_type != "video-game") {
            return Ok(Vec::new());
        }

        let options = GameSearchOptions::from(options.clone());
        self.search_games(query, &options).await
    }
}

pub fn all_querier_data_dir() -> Option<PathBuf> {
    match dirs::data_dir() {
        None => {
            None
        }
        Some(p) => {
            Some(p.join("all_querier"))
        }
    }
}
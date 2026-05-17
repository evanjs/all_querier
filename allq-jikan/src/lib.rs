use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use jikan_moe::common::response::Response;
use jikan_moe::common::structs::anime::Anime;
use jikan_moe::{JikanClient, JikanError};
use tracing::{debug, error};

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga", "character"];

/// A `SearchProvider` backed by the JikanProvider API.
pub struct JikanProvider {
    client: JikanClient,
    cache: Option<ProviderCache>
}

impl JikanProvider {
    /// Create a new provider with no cache.
    pub fn new() -> Self {
        Self {
            client: JikanClient::new(),
            cache: None
        }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(cache: ProviderCache) -> Self {
        Self {
            client: JikanClient::new(),
            cache: Some(cache),
        }
    }

    /// Look up a cached JSON string for `key`, respecting `fetch_mode`.
    ///
    /// Returns `Some(json)` on a cache hit (unless `ForceFetch`), `None` otherwise.
    async fn cache_get(&self, key: &str, fetch_mode: FetchMode) -> Option<String> {
        if fetch_mode == FetchMode::ForceFetch {
            return None;
        }
        let cache = self.cache.as_ref()?;
        cache.get(key).await.ok().flatten().map(|e| e.value().clone())
    }

    /// Insert `value` into the cache under `key` unless we are in `CacheOnly` mode
    /// (in which case there is nothing new to store).
    async fn cache_insert(&self, key: String, value: String, fetch_mode: FetchMode) {
        if fetch_mode == FetchMode::CacheOnly {
            return;
        }
        if let Some(cache) = &self.cache {
            cache.insert(key, value);
        }
    }
}

#[async_trait]
impl SearchProvider for JikanProvider {
    fn name(&self) -> &'static str {
        "jikan"
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let itype = item_type.unwrap_or("default_type");
        let limit = options.limit.unwrap_or(10);

        // Normalize list types to base types for search
        let normalized_itype = match itype {
            "animelist" => "anime",
            "mangalist" => "manga",
            _ => itype,
        };

        // When a media_type filter is active we need to over-fetch from the API
        // so that we still return `limit` results after the post-filter step.
        // MAL catalog search caps at 100 per page; user list endpoints support up to 1000.
        let api_limit: u32 = if options.media_type.is_some() {
            100
        } else {
            limit.min(100)
        };
        let mut results = Vec::new();

        const ANIME_FIELDS: &[&str] = &[
            "id", "title", "main_picture", "alternative_titles", "start_date", "end_date",
            "synopsis", "mean", "rank", "popularity", "num_list_users", "num_scoring_users",
            "nsfw", "media_type", "status", "genres", "num_episodes", "start_season",
            "broadcast", "source", "average_episode_duration", "rating", "studios",
        ];

        const MANGA_FIELDS: &[&str] = &[
            "id", "title", "main_picture", "alternative_titles", "start_date", "end_date",
            "synopsis", "mean", "rank", "popularity", "num_list_users", "num_scoring_users",
            "nsfw", "media_type", "status", "genres", "num_volumes", "num_chapters", "authors",
        ];

        // TODO: implement caching and adhere to user-specified caching mode
        //   e.g. force-fetch, cache-only, etc.
        //   This will probably be easier after deduplicating logic shared between
        //   search handlers

        debug!(
            ?query,
            nsfw =? options.nsfw,
            itype = %itype,
            limit = limit,
            search_type =? normalized_itype,
            "Searching jikan"
        );

        // TODO: deduplicate shared search params configuration

        match normalized_itype {
            "anime" => {
                let mut search_params = jikan_moe::anime::SearchParams::default();
                search_params.limit = Option::from(api_limit);
                // our option tracks whether we want to enable nsfw, while jikan_moe
                // tracks whether the search should only be sfw
                //
                // So we flip the option here as the intent is inverted
                search_params.sfw = Some(!options.nsfw);
                search_params.q = Some(query.into());

                debug!(
                    query =? search_params.q,
                    sfw =? search_params.sfw,
                    limit =? search_params.limit,
                    "Searching jikan for anime"
                );

                let anime_results = self.client
                    .get_anime_search(Some(search_params))
                    .await;

                let anime_results = match anime_results {
                    Ok(o) => {
                        o
                    },
                    Err(e) => {
                        error!(
                            error =? e,
                            "Encountered error when searching anime using Jikan"
                        );
                        return Err(anyhow!(e))
                    }
                };

                for node in anime_results.data {
                    results.push(SearchResult {
                        label: node.title.clone(),
                        id: node.mal_id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "jikan".to_string(),
                        description: node.synopsis.clone(),
                        data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                    });
                }

                if let Some(ref mt) = options.media_type {
                    results.retain(|r| {
                        r.data
                            .get("media_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case(mt))
                            .unwrap_or(false)
                    });
                }

                // Apply the user-requested limit after filtering so the final result
                // count is correct regardless of how many items were filtered out.
                results.truncate(limit as usize);

                Ok(results)
            },
            "manga" => {
                let mut search_params = jikan_moe::manga::SearchParams::default();
                search_params.limit = Option::from(api_limit);
                search_params.sfw = Some(!options.nsfw);
                let manga_results = self.client
                    .get_manga_search(Some(search_params))
                    .await?;

                for node in manga_results.data {
                    results.push(SearchResult {
                        label: node.title.clone(),
                        id: node.mal_id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "jikan".to_string(),
                        description: node.synopsis.clone(),
                        data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                    });
                }

                if let Some(ref mt) = options.media_type {
                    results.retain(|r| {
                        r.data
                            .get("media_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case(mt))
                            .unwrap_or(false)
                    });
                }

                // Apply the user-requested limit after filtering so the final result
                // count is correct regardless of how many items were filtered out.
                results.truncate(limit as usize);

                Ok(results)
            },
            "character" => {
                let character_results = self.client
                    .get_character_search(
                        None, // page
                        options.limit, // limit
                        Some(query.into()), // query
                        None, // order_by
                        None, // sort
                        None // letter
                    )
                    .await?;

                for node in character_results.data {
                    results.push(SearchResult {
                        label: node.name.clone(),
                        id: node.mal_id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "jikan".to_string(),
                        description: node.about.clone(),
                        data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                    });
                }

                if let Some(ref mt) = options.media_type {
                    results.retain(|r| {
                        r.data
                            .get("media_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case(mt))
                            .unwrap_or(false)
                    });
                }

                // Apply the user-requested limit after filtering so the final result
                // count is correct regardless of how many items were filtered out.
                results.truncate(limit as usize);

                Ok(results)
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported item type: {}", itype))
            }
        }
    }
}
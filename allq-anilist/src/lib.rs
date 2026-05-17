use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::{debug, error};
use anilist_moe::AniListClient;

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga", "character"];

/// A `SearchProvider` backed by the AniListProvider API.
pub struct AniListProvider {
    client: AniListClient,
    cache: Option<ProviderCache>
}

impl AniListProvider {
    /// Create a new provider with no cache.
    pub fn new() -> Self {
        Self {
            client: AniListClient::new(),
            cache: None
        }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(cache: ProviderCache) -> Self {
        Self {
            client: AniListClient::new(),
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
impl SearchProvider for AniListProvider {
    fn name(&self) -> &'static str {
        "anilist"
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
            "Searching AniList"
        );

        // TODO: deduplicate shared search params configuration

        match normalized_itype {
            "anime" => {
                let anime_results = self.client
                    .media()
                    .search_anime(
                        query,
                        Some(1),
                        Some(limit as i32)
                    )
                    .await;

                let anime_results = match anime_results {
                    Ok(o) => {
                        o
                    },
                    Err(e) => {
                        error!(
                            error =? e,
                            "Encountered error when searching anime using AniList"
                        );
                        return Err(anyhow!(e))
                    }
                };

                for node in anime_results.data {
                    results.push(SearchResult {
                        label: node.title.as_ref().unwrap().english.clone().unwrap_or("N/A".to_string()),
                        // TODO: Account for <NATIVE_ID> and <EXTERNAL_PROVIDER_ID>
                        //  e.g. in this case, AniList ID and MyAnimeList ID
                        id: node.id.map(|x|x.to_string()).or(Some("NO_ID".to_string())).unwrap(),
                        item_type: Some(itype.to_string()),
                        provider: "anilist".to_string(),
                        description: node.description.clone(),
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
                let manga_results = self.client
                    .media()
                    .search_manga(
                        query,
                        Some(1),
                        Some(limit as i32)
                    )
                    .await;

                let manga_results = match manga_results {
                    Ok(o) => {
                        o
                    },
                    Err(e) => {
                        error!(
                            error =? e,
                            "Encountered error when searching manga using AniList"
                        );
                        return Err(anyhow!(e))
                    }
                };

                for node in manga_results.data {
                    results.push(SearchResult {
                        label: node.title.as_ref().unwrap().english.clone().unwrap_or("N/A".to_string()),
                        // TODO: Account for <NATIVE_ID> and <EXTERNAL_PROVIDER_ID>
                        //  e.g. in this case, AniList ID and MyAnimeList ID
                        id: node.id.map(|x|x.to_string()).or(Some("NO_ID".to_string())).unwrap(),
                        item_type: Some(itype.to_string()),
                        provider: "anilist".to_string(),
                        description: node.description.clone(),
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
                    .character()
                    .search(
                        query,
                        Some(1),
                        Some(limit as i32)
                    )
                    .await;

                let character_results = match character_results {
                    Ok(o) => {
                        o
                    },
                    Err(e) => {
                        error!(
                            error =? e,
                            "Encountered error when searching character using AniList"
                        );
                        return Err(anyhow!(e))
                    }
                };

                for node in character_results.data {
                    results.push(SearchResult {
                        label: node.name.as_ref().unwrap().full.clone().unwrap_or("N/A".to_string()),
                        // TODO: Account for <NATIVE_ID> and <EXTERNAL_PROVIDER_ID>
                        //  e.g. in this case, AniList ID and MyAnimeList ID
                        id: node.id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "anilist".to_string(),
                        description: node.description.clone(),
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
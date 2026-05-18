use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::{anyhow, Context, Error, Result};
use async_trait::async_trait;
use tracing::{debug, error};
use anilist_moe::AniListClient;
use anilist_moe::endpoints::character::FetchCharacterOptions;
use anilist_moe::endpoints::media::FetchMediaOptions;
use anilist_moe::objects::media::Media;
use anilist_moe::objects::responses::Page;

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga", "character", "person"];

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

    fn get_default_fetch_media_options(&self) -> FetchMediaOptions {
        let mut options = FetchMediaOptions::default();
        options.include_external_links = Some(true);
        options.include_country_of_origin = Some(true);
        options.include_duration = Some(true);
        options.include_is_licensed = Some(true);
        options.include_hashtag = Some(true);
        options.include_end_date = Some(true);
        options.include_start_date = Some(true);
        options.include_studios = Some(true);
        options.include_tags = Some(true);
        options.include_volumes = Some(true);
        options.include_streaming_episodes = Some(true);
        options.include_source = Some(true);
        // Less static values
        // Not sure if it's "correct" to include these by default
        // Especially re: accuracy of "cached" values
        // options.include_next_airing_episode = Some(true);
        //
        // Values that might require pagination
        // options.include_characters = Some(true);
        // options.include_relations = Some(true);
        // options.include_staff = Some(true);
        options
    }

    fn process_media_list_into_results(itype: &str, output_results: &mut Vec<SearchResult>, media_results: &Page<Vec<Media>>) {
        media_results.data.iter().for_each(|node| {
            output_results.push(SearchResult {
                label: node.title.as_ref().unwrap().english.clone().unwrap_or("N/A".to_string()),
                // TODO: Account for <NATIVE_ID> and <EXTERNAL_PROVIDER_ID>
                //  e.g. in this case, AniList ID and MyAnimeList ID
                id: node.id.map(|x| x.to_string()).or(Some("NO_ID".to_string())).unwrap(),
                item_type: Some(itype.to_string()),
                provider: "anilist".to_string(),
                description: node.description.clone(),
                data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
            });
        });
    }

    fn filter_collection_by_media_type(options: &SearchOptions, results: &mut Vec<SearchResult>) {
        if let Some(ref mt) = options.media_type {
            results.retain(|r| {
                r.data
                    .get("media_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case(mt))
                    .unwrap_or(false)
            });
        }
    }

    async fn process_anilist_media_query(
        &self,
        query: &str,
        options: &SearchOptions,
        itype: &str,
        limit: u32,
        mut results: &mut Vec<SearchResult>
    ) -> Result<(), Error> {
        debug!(
            ?query,
            ?itype,
            ?limit,
            ?options,
            "Processing search query for AniList"
        );

        let media_type = itype;

        match media_type {
            // both anime and manga use the "media" endpoint
            "anime" | "manga" => {
                let media_results_pre = self.client
                    .media();

                let mut fetch_options = self.get_default_fetch_media_options();
                fetch_options.search = Some(query.to_string());
                fetch_options.per_page = Some(options.limit.unwrap_or(10u32) as i32);
                fetch_options.page = Some(1);

                let media_results = match media_type {
                    "anime" => {
                        fetch_options.media_type = Some(anilist_moe::enums::media::MediaType::Anime);
                        media_results_pre.fetch(
                            &fetch_options
                        )
                            .await
                    }
                    "manga" => {
                        fetch_options.media_type = Some(anilist_moe::enums::media::MediaType::Manga);
                        media_results_pre.fetch(
                          &fetch_options
                        )
                            .await
                    }
                    // XXX: Not sure why we need to handle fallthrough if the parent match already
                    //   accounts for only "anime" or "manga"
                    //   Perhaps it is because the variables are not the "same"?
                    _ => {
                        error!(
                            media_type =? media_type,
                            "Unsupported media type"
                        );
                        return Err(anyhow!("Unsupported media type a"));
                    }
                };

                let media_results = match media_results {
                    Ok(o) => o,
                    Err(e) => {
                        error!(
                            error =? e,
                            ?media_type,
                            "Encountered error when searching AniList"
                        );
                        return Err(anyhow!(e));
                    }
                };

                Self::process_media_list_into_results(itype, &mut results, &media_results);

                Self::filter_collection_by_media_type(options, &mut results);

                // Apply the user-requested limit after filtering so the final result
                // count is correct regardless of how many items were filtered out.
                let mut data = media_results.data.clone();
                data.truncate(limit as usize);

                Ok(())
            }
            _ => {
                error!(
                    media_type =? media_type,
                    "Unsupported media type"
                );
                Err(anyhow!("Unsupported media type b"))
            }
        }
    }

    async fn process_anilist_staff_query(
        &self,
        query: &str,
        options: &SearchOptions,
        itype: &str,
        limit: u32,
        mut results: &mut Vec<SearchResult>
    ) -> Result<(), Error> {
        debug!(
            ?query,
            ?options,
            ?itype,
            ?limit,
            "Searching for staff on AniList"
        );
        let staff_results = self.client
            .staff()
            .search(
                query, // query
                Some(1), // page number
                Some(limit as i32) // number of results per page
            )
            .await;

        let staff_results = match staff_results {
            Ok(o) => o,
            Err(e) => {
                error!(
                    error =? e,
                    "Encountered error when searching staff using AniList"
                );
                return Err(anyhow!(e))
            }
        };

        for node in staff_results.data {
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

        Self::filter_collection_by_media_type(options, &mut results);

        // Apply the user-requested limit after filtering so the final result
        // count is correct regardless of how many items were filtered out.
        results.truncate(limit as usize);

        Ok(())
    }
    async fn process_anilist_character_query(
        &self, query: &str,
        options: &SearchOptions,
        itype: &str,
        limit: u32,
        mut results: &mut Vec<SearchResult>
    ) -> Result<(), Error> {
        let mut fetch_options = FetchCharacterOptions::default();
        fetch_options.search = Some(query.to_string());
        let character_results = self.client
            .character()
            .fetch(&fetch_options)
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

        Self::filter_collection_by_media_type(options, &mut results);

        // Apply the user-requested limit after filtering so the final result
        // count is correct regardless of how many items were filtered out.
        results.truncate(limit as usize);

        Ok(())
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
            "person" => "staff",
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

        let fetch_mode = options.fetch_mode;
        // default to one for "single search" mode
        let limit = options.limit.unwrap_or(1);
        let search_cache_key = format!("anilist:search:{normalized_itype}:{query}:{limit}");
        debug!(
            ?query,
            ?fetch_mode,
            ?limit,
            ?search_cache_key
        );

        // Try to serve the full result list from cache.
        if let Some(cached) = self.cache_get(&search_cache_key, fetch_mode).await {
            let value: serde_json::Value = serde_json::from_str(&cached)
                .context("failed to deserialize cached AniList search results")?;
            let results: Vec<SearchResult> = serde_json::from_value(value)
                .context("failed to convert cached AniList search results")?;
            return Ok(results);
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        debug!(
            ?query,
            nsfw =? options.nsfw,
            itype = %itype,
            limit = limit,
            search_type =? normalized_itype,
            "Searching AniList"
        );

        let result = match normalized_itype {
            "anime" | "manga" => {
                match self.process_anilist_media_query(query, options, itype, api_limit, &mut results).await {
                    Ok(_) => {
                        Ok(results)
                    },
                    Err(value) => Err(value),
                }
            },
            "character" => {
                match self.process_anilist_character_query(query, options, itype, api_limit, &mut results).await {
                    Ok(_) => Ok(results),
                    Err(value) => Err(value)
                }
            },
            "staff" => {
                match self.process_anilist_staff_query(query, options, itype, api_limit, &mut results).await {
                    Ok(_) => Ok(results),
                    Err(value) => Err(value)
                }
            },
            _ => {
                Err(anyhow::anyhow!("Unsupported item type: {} (normalized: {})", itype, normalized_itype))
            }
        }?;

        // Cache the assembled result list.searc
        if let Ok(json) = serde_json::to_string(&result) {
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(result)
    }
}
mod oauth2;

use std::{env, fs};
use std::num::ParseIntError;
use allq_core::{all_querier_cache_dir, FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::{anyhow, Context, Error, Result};
use async_trait::async_trait;
use tracing::{debug, error, info, trace, warn};
use anilist_moe::AniListClient;
use anilist_moe::endpoints::character::FetchCharacterOptions;
use anilist_moe::endpoints::media::FetchMediaOptions;
use anilist_moe::endpoints::MediaListEndpoint;
use anilist_moe::endpoints::user::FetchUserMediaListOptions;
use anilist_moe::enums::media::MediaType;
use anilist_moe::enums::media_list::MediaListStatus;
use anilist_moe::objects::common::PageInfo;
use anilist_moe::objects::media::Media;
use anilist_moe::objects::responses::Page;
use serde::Deserialize;
use crate::oauth2::{get_oauth2_token, wait_for_oauth2_input};

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga", "character", "person", "animelist", "mangalist"];


#[derive(Deserialize, Debug, Default)]
struct AniListConfig {
    client_id: Option<u32>,
    client_secret: Option<String>,
    access_token: Option<String>,
}

pub fn get_config() -> Result<AniListConfig> {
    let mut access_token = 0;
    match env::var("ANILIST_TOKEN") {
        Ok(token) => {
            match token.parse::<u32>() {
                Ok(parsed_token) => {
                    access_token = parsed_token;
                }
                Err(e) => {
                    warn!("Failed to parse token ({token}) as u32: {e}");
                }
            };
        },
        Err(e) => {
            debug!("Failed to retrieve ANILIST_TOKEN from environment: {e}");
        }
    };

    trace!("Attempting to parse AniList configuration file");
    let path = all_querier_cache_dir().join("anilist_config.json");
    let config = if path.exists() {
        let content = fs::read_to_string(&path)?;
        let config: AniListConfig = serde_json::from_str(&content)?;
        config
    } else {
        AniListConfig {
            ..Default::default()
        }
    };

    return Ok(config);

    anyhow::bail!("Please set ANILIST_TOKEN in environment or in {:?}", path);
}


/// A `SearchProvider` backed by the AniListProvider API.
pub struct AniListProvider {
    client: AniListClient,
    cache: Option<ProviderCache>,
}

impl AniListProvider {
    /// Create a new provider with no cache.
    // TODO: perhaps this would be better as a default since there are no parameters
    //   then we might allow optional parameters in this function (`new`), perhaps?
    pub fn new() -> Self {
        let mut client = AniListClient::new();
        match get_config() {
            Ok(config) => {
                match config.access_token {
                    None => {
                        warn!("No access token provided. Fetching ...");
                        match get_oauth2_token(config.client_id) {
                            Ok(csrf_token) => {
                                match wait_for_oauth2_input(&csrf_token) {
                                    Ok(token) => {
                                        client.set_token(&token);
                                    },
                                    Err(e) => {
                                        error!("Failed to complete OAUTH2 authentication process: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to retrieve access token for user. Falling back to unauthenticated mode")
                            }
                        }

                    }
                    Some(access_token) => {
                        client.set_token(&access_token);
                    }
                }
            },
            Err(e) => {
                warn!("Failed to retrieve AniList configuration: {e}");
            }
        }

        Self {
            client,
            cache: None
        }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(cache: ProviderCache) -> Self {
        let mut anilist_provider = Self::new();
        anilist_provider.cache = Some(cache);

        anilist_provider
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
        fetch_options.include_media = Some(true);
        fetch_options.per_page = Option::from(limit as i32);
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

    async fn process_anilist_user_query(
        &self,
        options: &SearchOptions,
        itype: &str,
        limit: u32,
        mut results: &mut Vec<SearchResult>
    ) -> Result<(), Error> {
        assert_ne!(options.anilist_username, None, "\"anilist_username\" is empty but must be set");
        debug!(
            client =? self.client,
        );
        let user_info = self.client.user().get_current_user().await?;
        let username = user_info.name.as_ref().unwrap();
        let medialist = self.client.medialist();
        debug!(
            ?user_info,
            "Info for current authenticated user"
        );
        let effective_limit = Some(limit as i32);
        let mut page_number = 1;
        let status = None;
        debug!(
            ?effective_limit,
            ?page_number
        );

        // process first page
        let first_page_result = Self::process_user_medialist_page(
            itype,
            &mut results,
            username,
            &medialist,
            effective_limit,
            Some(page_number),
            status
        ).await?;
        debug!(
            ?status,
            ?page_number,
            ?username,
            ?itype,
            ?effective_limit,
            page_info =? &first_page_result.page_info,
            "First page results"
        );


        let total_items = first_page_result.data.len() as i32;
        let mut processed_items = total_items;

        let mut iterating = matches!(first_page_result.page_info, Some(info) if info.has_next_page.unwrap_or(false));

        const ANILIST_RATE_LIMIT_MAX_REQUESTS: usize = 30;
        const ANILIST_RATE_LIMIT_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);
        const ANILIST_RATE_LIMIT_BUFFER: std::time::Duration = std::time::Duration::from_millis(250);

        let mut request_timestamps = std::collections::VecDeque::new();

        // Count the first page request, which already happened immediately before this block.
        request_timestamps.push_back(std::time::Instant::now());

        // if total items in the list does not exceed the user-defined limit (or default limit)
        // then attempt to iterate the list further
        while iterating {
            if Some(processed_items) < effective_limit {
                info!("Processed items ({}) does not exceed that of effective limit ({:?}). Continuing ...", processed_items, effective_limit);
                page_number += 1;

                loop {
                    let now = std::time::Instant::now();

                    while request_timestamps
                        .front()
                        .is_some_and(|timestamp| now.duration_since(*timestamp) >= ANILIST_RATE_LIMIT_WINDOW)
                    {
                        request_timestamps.pop_front();
                    }

                    if request_timestamps.len() < ANILIST_RATE_LIMIT_MAX_REQUESTS {
                        request_timestamps.push_back(now);
                        break;
                    }

                    let oldest_request = *request_timestamps
                        .front()
                        .expect("rate-limit queue should not be empty when limit is reached");

                    let sleep_for = ANILIST_RATE_LIMIT_WINDOW
                        .saturating_sub(now.duration_since(oldest_request))
                        + ANILIST_RATE_LIMIT_BUFFER;

                    warn!(
                            "AniList API rolling rate limit reached ({}/{} requests). Waiting {:?} before continuing ...",
                            request_timestamps.len(),
                            ANILIST_RATE_LIMIT_MAX_REQUESTS,
                            sleep_for
                        );

                    tokio::time::sleep(sleep_for).await;
                }

                // then second page and onward if needed
                let next_page = Self::process_user_medialist_page(
                    itype,
                    &mut results,
                    username,
                    &medialist,
                    effective_limit,
                    Some(page_number),
                    status,
                ).await?;

                let more_items = next_page.data.len() as i32;
                processed_items += more_items;

                iterating = matches!(next_page.page_info, Some(info) if info.has_next_page.unwrap_or(false));
            } else {
                // if we have exceeded the limit, short circuit and return
                warn!("Processed items ({}) exceeds that of effective limit ({:?}). Short circuiting ...", processed_items, effective_limit);
                break
            }
        }
        Ok(())
    }

    async fn process_user_medialist_page(
        itype: &str,
        results: &mut Vec<SearchResult>,
        username: &String,
        medialist: &MediaListEndpoint,
        effective_limit: Option<i32>,
        page_number: Option<i32>,
        status: Option<MediaListStatus>
    ) -> Result<Page<Vec<anilist_moe::objects::media_list::MediaList>>> {
        debug!(
            page_number =? page_number.as_ref(),
            media_type =? itype,
            "Retrieving page for medialist"
        );
        let medialist_results = match itype {
            "animelist" => {
                medialist
                    .get_user_anime_list(username, status, page_number, effective_limit)
                    .await
            }
            "mangalist" => {
                medialist
                    .get_user_manga_list(username, status, page_number, effective_limit)
                    .await
            },
            _ => panic!("Unrecognized itype input. Must be either \"animelist\" or \"mangalist\"")
        };

        trace!(
            intermediate_medialist_results =? medialist_results,
        );

        let mut medialist_results = match medialist_results {
            Ok(o) => {
                trace!(
                    page =? &o,
                );
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

        trace!(
            ?medialist_results,
            "Got medialist results"
        );

        for node in &medialist_results.data {
            let title = node.media.as_ref().map(|media|
                media.title.clone().map(|t|
                    t.user_preferred.or(
                        t.english.or(
                            t.romaji.as_ref().or(t.native.as_ref()).cloned()
                        )
                    )
                )
            )
                .flatten().flatten().unwrap_or_else(|| "Untitled media".to_string());
            results.push(SearchResult {
                label: title.clone(),
                // TODO: Account for <NATIVE_ID> and <EXTERNAL_PROVIDER_ID>
                //  e.g. in this case, AniList ID and MyAnimeList ID
                id: node.id.to_string(),
                item_type: Some(itype.to_string()),
                provider: "anilist".to_string(),
                description: Some(node.media.as_ref().unwrap().description.as_ref().unwrap_or(&"N/A".to_string()).to_string()),
                data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
            });
        }
        Ok(medialist_results)
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
        let api_limit = match itype {
            "animelist" | "mangalist" => 1000,
            _ => {
                if options.media_type.is_some() {
                    100
                } else {
                    limit.min(100)
                }
            }
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
        let search_cache_key = format!("anilist:search:{itype}:{query}:{limit}");
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
        } else {
            warn!(
                ?search_cache_key,
                ?fetch_mode,
                "Failed to find item in cache"
            );
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

        let result = match itype {
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
            "person" => {
                match self.process_anilist_staff_query(query, options, itype, api_limit, &mut results).await {
                    Ok(_) => Ok(results),
                    Err(value) => Err(value)
                }
            },
            "animelist" | "mangalist" => {
                match self.process_anilist_user_query(options, itype, api_limit, &mut results).await {
                    Ok(_) => Ok(results),
                    Err(value) => Err(value)
                }
            }
            _ => {
                Err(anyhow::anyhow!("Unsupported item type: {} (normalized: {})", itype, normalized_itype))
            }
        }?;

        // Cache the assembled result list
        if let Ok(json) = serde_json::to_string(&result) {
            trace!(
                ?search_cache_key,
                "Inserting item into cache"
            );
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(result)
    }
}
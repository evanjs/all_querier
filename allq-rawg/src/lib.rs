#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate serde_repr;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate reqwest;

use std::{env, fs};
use std::collections::VecDeque;
use std::sync::Mutex;
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, trace};
use allq_core::{all_querier_cache_dir, all_querier_data_dir, FetchMode, GameSearchOptions, GameSearchProvider, ProviderCache, SearchOptions, SearchResult};
use crate::apis::configuration::Configuration;

pub mod apis;
pub mod models;

pub const SUPPORTED_TYPES: &[&str] = &["video-game"];
pub(crate) const LINK_ALIASES: &[&str] = &[
    "rawg",
];

/// A `SearchProvider` backed by the RawgProvider API.
pub struct RawgProvider {
    client: reqwest::Client,
    cache: Option<ProviderCache>,
}

fn user_agent() -> String {
    concat!(
        env!("CARGO_PKG_NAME"),
        "/",
        env!("CARGO_PKG_VERSION")
    ).to_string()
}

impl RawgProvider {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent())
            .build()?;

        Ok(Self { client, cache: None })
    }


    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(cache: ProviderCache) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(user_agent())
            .build()?;

        Ok(Self { client, cache: Some(cache) })
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
}

#[derive(Deserialize)]
struct RawgConfig {
    rawg_api_key: Option<String>,
}

pub fn get_config() -> Result<RawgConfig> {
    if let Ok(rawg_api_key) = env::var("RAWG_API_KEY") {
        return Ok(RawgConfig {
            rawg_api_key: Some(rawg_api_key)
        });
    }

    let path = all_querier_data_dir()
        .expect("Failed to get all querier data directory")
        .join("rawg")
        .join("config.json");
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let config: RawgConfig = serde_json::from_str(&content)?;
        return Ok(config);
    }

    anyhow::bail!("Please set RAWG_API_KEY in environment or in {:?}", path)
}

#[async_trait]
impl GameSearchProvider for RawgProvider {
    fn name(&self) -> &'static str {
        "rawg"
    }

    async fn search_games(
        &self,
        query: &str,
        options: &GameSearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let itype = "video-game";
        let limit = options.limit.unwrap_or(10);

        debug!(
            ?query,
            itype = %itype,
            limit = limit,
            "Searching RAWG"
        );

        let fetch_mode = options.fetch_mode;
        let config = get_config()?;
        let search_cache_key = format!("rawg:search:{itype}:{query}:{limit}");

        // Try to serve the full result list from cache.
        if let Some(cached) = self.cache_get(&search_cache_key, fetch_mode).await {
            let value: serde_json::Value = serde_json::from_str(&cached)
                .context("failed to deserialize cached RAWG search results")?;
            let results: Vec<SearchResult> = serde_json::from_value(value)
                .context("failed to convert cached RAWG search results")?;
            return Ok(results);
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let results = match config.rawg_api_key {
            Some(api_key) => {
                let rawg_config = &Configuration {
                    client: self.client.clone(),
                    user_agent: Some(user_agent()),
                    api_key: Some(apis::configuration::ApiKey {
                        key: api_key,
                        prefix: None
                    }),
                    ..Default::default()
                };

                debug!(
                    ?rawg_config,
                );

                apis::games_api::games_list(
                    rawg_config,
                    None, // page
                    options.limit.map(|x|x as i32), // page_size
                    Some(query), // search
                    None, // search_precise
                    None, // search_exact
                    None, // parent_platforms
                    None, // platforms
                    None, // stores
                    None, // developers
                    None, // publishers
                    None, // genres
                    None, // tags
                    None, // creators
                    None, // dates
                    None, // updated
                    None, // platforms_count
                    None, // metacritic
                    None, // exclude_collection
                    None, // exclude_additions
                    None, // exclude_parents
                    None, // exclude_game_series
                    None, // exclude_stores
                    None, // ordering
                ).await?
            },
            None => {
                // No API key found, bail out
                anyhow::bail!("Failed to find RAWG API key. Exiting ...");
            }
        };

        // TODO: handle pagination
        let search_results = results.results.iter().map(|game| {
            let game_value = serde_json::to_value(game)
                .expect("Failed to convert RAWG game to JSON");
            SearchResult {
                provider: "rawg".to_string(),
                id: game.id.as_ref().expect("Failed to find game ID").to_string(),
                label: game.name.as_ref().expect("Failed to find game name").to_string(),
                description: game.slug.clone(),
                item_type: Some("video-game".to_string()),
                data: game_value,
            }
        }).collect::<Vec<_>>();

        // Cache the assembled result list.
        if let Ok(json) = serde_json::to_string(&search_results) {
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(search_results)
    }
}
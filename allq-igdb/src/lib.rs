#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_repr;
extern crate url;

use allq_core::{all_querier_cache_dir, all_querier_data_dir, FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use igdb_atlas::endpoints::traits::{Endpoint, NameFilterable, Searchable};
use igdb_atlas::{ClientConfig, IGDBClient, IGDBError, QueryBuilder};
use serde::Deserialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::{env, fs};
use tracing::{debug, error, trace};

pub const SUPPORTED_TYPES: &[&str] = &["video-game"];
pub(crate) const LINK_ALIASES: &[&str] = &[
    "igdb",
];

/// A `SearchProvider` backed by the IGDBProvider API.
pub struct IGDBProvider {
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

impl IGDBProvider {
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

    async fn search_game(&self, client: &IGDBClient, query: &str, search_options: &SearchOptions) -> Result<Vec<igdb_atlas::Game>> {
        // TODO: make this customizable
        client
            .games()
            .search(&query)
            .limit(search_options.limit.unwrap_or(10))
            .execute()
            .await
            .inspect_err(|e| {
                error!(
                    error =? e,
                    "Failed to search using IGDB atlas"
                )
            }).map_err(|e| {
            e.into()
        })
    }
}

#[derive(Debug, Deserialize)]
struct IGDBConfig {
    igdb_api_key: Option<String>,
    igdb_client_id: Option<String>,
    igdb_client_secret: Option<String>,
}

pub fn get_local_config() -> Result<IGDBConfig> {
    match (env::var("IGDB_API_KEY"), env::var("IGDB_CLIENT_ID"), env::var("IGDB_CLIENT_SECRET")) {
        (Ok(igdb_api_key), Ok(igdb_client_id), _) => {
            return Ok(IGDBConfig {
                igdb_api_key: Some(igdb_api_key),
                igdb_client_id: Some(igdb_client_id),
                igdb_client_secret: None
            });
        },
        (_, Ok(igdb_client_id), Ok(igdb_client_secret)) => {
            return Ok(IGDBConfig {
                igdb_api_key: None,
                igdb_client_id: Some(igdb_client_id),
                igdb_client_secret: Some(igdb_client_secret)
            });
        }
        _ => {}
    }

    let path = all_querier_data_dir()
        .expect("Failed to get all querier data directory")
        .join("igdb")
        .join("config.json");
    debug!(
        ?path,
        "Checking for IGDB config"
    );
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        debug!(
            ?content,
            "Config file exists. Reading ..."
        );
        let config: IGDBConfig = serde_json::from_str(&content)?;
        debug!(
            ?config,
            "Read config file. Returning ..."
        );
        return Ok(config);
    }

    anyhow::bail!("Failed to parse IGDB configuration")
}


impl TryInto<ClientConfig> for IGDBConfig {
    type Error = IGDBError;

    fn try_into(self) -> std::result::Result<ClientConfig, Self::Error> {
        match (self.igdb_client_id, self.igdb_client_secret) {
            (Some(client_id), Some(client_secret)) => {
                let config = ClientConfig::builder()
                    .client_id(&client_id)
                    .client_secret(&client_secret)
                    .build()
                    .expect("Failed to construct IGDB Client Config");
                Ok(config)
            },
            (None, Some(_)) => {
                let error = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Client ID was not found in local configuration"
                );
                Err(IGDBError::from_custom(error))
            },
            (Some(_), None) => {
                let error = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Client Secret token was not found in local configuration"
                );
                Err(IGDBError::from_custom(error))
            },
            _ => {
                let error = std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Neither Client ID nor Client Secret were not found in local configuration"
                );
                Err(IGDBError::from_custom(error))
            }
        }
    }
}

#[async_trait]
impl SearchProvider for IGDBProvider {
    fn name(&self) -> &'static str {
        "igdb"
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

        debug!(
            ?query,
            itype = %itype,
            limit = limit,
            "Searching IGDB"
        );

        let fetch_mode = options.fetch_mode;
        let config = get_local_config()?;
        debug!(
            ?config
        );
        let i_config = config
            .try_into()
            .expect("Failed to get IGDB atlas configuration from local configuration");
        let client = IGDBClient::new(i_config).await?;
        let search_cache_key = format!("igdb:v2:search:{itype}:{query}:{limit}");

        // Try to serve the full result list from cache.
        if let Some(cached) = self.cache_get(&search_cache_key, fetch_mode).await {
            let value: serde_json::Value = serde_json::from_str(&cached)
                .context("failed to deserialize cached IGDB search results")?;
            let results: Vec<SearchResult> = serde_json::from_value(value)
                .context("failed to convert cached IGDB search results")?;
            return Ok(results);
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let results = self.search_game(&client, query, options).await?;

        // TODO: handle pagination
        let search_results = results.iter().map(|game| {
            let game_value = serde_json::to_value(game)
                .expect("Failed to convert IGDB game to JSON");
            SearchResult {
                provider: "igdb".to_string(),
                id: game.id.to_string(),
                label: game.name.as_ref().unwrap_or(&"No label".to_string()).to_string(),
                description: game.summary.clone(),
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
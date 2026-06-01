#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate serde_repr;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate reqwest;

use std::{env, fs};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, trace};
use allq_core::{all_querier_cache_dir, all_querier_data_dir, SearchOptions, SearchProvider, SearchResult};
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

        Ok(Self { client })
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
impl SearchProvider for RawgProvider {
    fn name(&self) -> &'static str {
        "rawg"
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
            "Searching RAWG"
        );

        let config = get_config()?;
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

        Ok(search_results)
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }
}
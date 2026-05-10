use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::Context;
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::debug;

/// Cargo tables to query and the fields to request from each.
/// Fields are the exact names as returned by `action=cargofields`.
static CARGO_TABLES: &[(&str, &str)] = &[
    (
        "Infobox_game",
        "Developers,Publishers,Engines,Released_Windows,Released_OS_X,Released_Linux,\
         Monetization,Microtransactions,Modes,Pacing,Perspectives,Controls,Genres,Sports,\
         Vehicles,Art_styles,Themes,Series,Steam_AppID,GOGcom_ID,Wikipedia,License",
    ),
    (
        "Taxonomy",
        "Category,Glossary",
    ),
    (
        "Availability",
        "Available_from,Uses_DRM,Steam_DRM,GOGcom_DRM,Epic_Games_Store_DRM,\
         Microsoft_Store_DRM,Xbox_Game_Pass,EA_Play,Apple_Arcade",
    ),
    (
        "Video",
        "Widescreen_resolution,Ultrawidescreen,4K_Ultra_HD,Windowed,\
         Borderless_fullscreen_windowed,Anisotropic_filtering,Antialiasing,Upscaling,\
         Frame_gen,Vsync,60_FPS,120_FPS,HDR,Ray_tracing,Field_of_view,Color_blind",
    ),
    (
        "Audio",
        "Separate_volume_controls,Surround_sound,Subtitles,Closed_captions,Mute_on_focus_lost",
    ),
    (
        "Input",
        "Full_controller_support,Controller_support,Key_remapping,Controller_remapping,\
         Mouse_acceleration,Mouse_sensitivity,Mouse_input_in_menus,\
         Controller_hotplugging,Simultaneous_input,Steam_Input_API_support",
    ),
    (
        "Multiplayer",
        "Local,Local_players,Local_modes,LAN,LAN_players,LAN_modes,\
         Online,Online_players,Online_modes,Asynchronous,Crossplay,Crossplay_platforms",
    ),
    (
        "Cloud",
        "Steam,GOG_Galaxy,Epic_Games_Launcher,Xbox",
    ),
    (
        "API",
        "Direct3D_versions,Vulkan_versions,OpenGL_versions,Metal_support,\
         Windows_64bit_executable,Windows_32bit_executable,\
         macOS_Intel_64bit_app,macOS_ARM_app,Linux_64bit_executable,Linux_ARM_app",
    ),
    (
        "Engine",
        "Developer,Website,First_release,Latest_release",
    ),
    (
        "Middleware",
        "Physics,Audio,Interface,Input,Cutscenes,Multiplayer,Anticheat",
    ),
];

pub const SUPPORTED_TYPES: &[&str] = &["video-game"];

/// Returns the canonical PCGW wiki page URL for a given page title/slug.
pub fn page_url(title: &str) -> String {
    format!("https://www.pcgamingwiki.com/wiki/{title}")
}

/// Returns the MediaWiki parse API URL for a given page title/slug.
pub fn parse_api_url(title: &str) -> String {
    format!(
        "https://www.pcgamingwiki.com/w/api.php\
         ?action=parse&redirects=1&page={}&format=json",
        urlencoding::encode(title),
    )
}

/// Extracts the `parse` object from a raw MediaWiki parse API JSON response body.
/// Returns an error if the response contains an API-level error or is missing the `parse` field.
pub fn parse_page_response(body: &str) -> anyhow::Result<Value> {
    let root: Value = serde_json::from_str(body)
        .context("failed to parse PCGW API response as JSON")?;
    if let Some(err) = root.get("error") {
        anyhow::bail!("PCGW API error: {err}");
    }
    root.get("parse")
        .cloned()
        .context("PCGW API response missing 'parse' field")
}

/// PCGW allows at most 20 requests per minute.
const RATE_LIMIT_MAX_REQUESTS: usize = 20;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

/// A `SearchProvider` backed by the PCGamingWiki MediaWiki search API.
pub struct PcgwSearchProvider {
    client: reqwest::Client,
    cache: Option<ProviderCache>,
    /// Timestamps of recent requests, used to enforce the 20 req/min burst limit.
    request_times: Mutex<VecDeque<Instant>>,
}

impl PcgwSearchProvider {
    pub fn new(user_agent: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .expect("failed to build PCGW HTTP client");
        Self { client, cache: None, request_times: Mutex::new(VecDeque::new()) }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(user_agent: &str, cache: ProviderCache) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .expect("failed to build PCGW HTTP client");
        Self { client, cache: Some(cache), request_times: Mutex::new(VecDeque::new()) }
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

    /// Enforce the 20 req/min rate limit, then perform a GET and return the parsed JSON body.
    /// Every outbound HTTP request to PCGW must go through this method.
    async fn rate_limited_get(&self, url: &str) -> anyhow::Result<Value> {
        let sleep_for = {
            let mut times = self.request_times.lock().unwrap();
            let now = Instant::now();
            while times.front().map_or(false, |t| now.duration_since(*t) >= RATE_LIMIT_WINDOW) {
                times.pop_front();
            }
            if times.len() >= RATE_LIMIT_MAX_REQUESTS {
                let oldest = *times.front().unwrap();
                let elapsed = now.duration_since(oldest);
                RATE_LIMIT_WINDOW.checked_sub(elapsed)
            } else {
                None
            }
        };
        if let Some(delay) = sleep_for {
            tokio::time::sleep(delay).await;
        }
        self.request_times.lock().unwrap().push_back(Instant::now());

        self.client
            .get(url)
            .send()
            .await
            .context("PCGW request failed")?
            .error_for_status()
            .context("PCGW returned error status")?
            .json::<Value>()
            .await
            .context("failed to parse PCGW JSON response")
    }

    /// Query all Cargo tables for `page_title` sequentially, merging results into a single object.
    /// Returns a JSON object keyed by table name, each value being the first row's `title` object.
    /// Results are cached under `pcgw:cargo:<title>`.
    async fn cargo_query_page(
        &self,
        page_title: &str,
        fetch_mode: FetchMode,
    ) -> anyhow::Result<Value> {
        let cache_key = format!("pcgw:cargo:{page_title}");

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            return serde_json::from_str(&cached)
                .context("failed to deserialize cached PCGW cargo data");
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Value::Object(Map::new()));
        }

        let encoded = urlencoding::encode(page_title);
        let mut merged = Map::new();

        for (table, fields) in CARGO_TABLES {
            let url = format!(
                "https://www.pcgamingwiki.com/w/api.php\
                 ?action=cargoquery&format=json&limit=5\
                 &tables={table}&fields={fields}&where=_pageName%3D%27{encoded}%27",
            );
            debug!(%url, table, "querying PCGW Cargo table");

            match self.rate_limited_get(&url).await {
                Ok(body) => {
                    // Extract the first row's `title` object from `cargoquery[0].title`
                    let row = body
                        .pointer("/cargoquery/0/title")
                        .cloned()
                        .unwrap_or(Value::Null);
                    merged.insert(table.to_string(), row);
                }
                Err(e) => {
                    debug!(error = %e, table, "PCGW Cargo table query failed, skipping");
                    merged.insert(table.to_string(), Value::Null);
                }
            }
        }

        let result = Value::Object(merged);
        if let Ok(json) = serde_json::to_string(&result) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }
        Ok(result)
    }

    /// Fetch full page data via the `parse` action, following redirects.
    /// Checks the cache first (keyed by title), stores the result on a miss.
    async fn parse_page(&self, title: &str, fetch_mode: FetchMode) -> anyhow::Result<Value> {
        let cache_key = format!("pcgw:parse:{title}");

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            return serde_json::from_str(&cached)
                .context("failed to deserialize cached PCGW parse response");
        }

        if fetch_mode == FetchMode::CacheOnly {
            anyhow::bail!("PCGW parse for '{title}' not in cache (cache-only mode)");
        }

        let url = parse_api_url(title);
        debug!(%url, "parsing PCGW page");

        let raw: Value = self.rate_limited_get(&url).await
            .context("PCGW parse request failed")?;
        let body = serde_json::to_string(&raw)
            .context("failed to re-serialize PCGW parse response")?;
        let value = parse_page_response(&body)?;

        if let Ok(json) = serde_json::to_string(&value) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }

        Ok(value)
    }
}

#[async_trait]
impl SearchProvider for PcgwSearchProvider {
    fn name(&self) -> &'static str {
        "pcgw"
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        // PCGW only indexes video games — skip if the caller asked for a different type.
        if let Some(t) = item_type {
            if t != "video-game" {
                return Ok(Vec::new());
            }
        }

        let fetch_mode = options.fetch_mode;
        let limit = options.limit.unwrap_or(10).min(50);
        let search_cache_key = format!("pcgw:search:{query}:{limit}");

        // Try to serve the full result list from cache.
        if let Some(cached) = self.cache_get(&search_cache_key, fetch_mode).await {
            let value: serde_json::Value = serde_json::from_str(&cached)
                .context("failed to deserialize cached PCGW search results")?;
            let results: Vec<SearchResult> = serde_json::from_value(value)
                .context("failed to convert cached PCGW search results")?;
            return Ok(results);
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let url = format!(
            "https://www.pcgamingwiki.com/w/api.php\
             ?action=query&list=search&srsearch={}&srlimit={}&format=json",
            urlencoding::encode(query),
            limit,
        );

        debug!(%url, "searching PCGW");

        let body: Value = self.rate_limited_get(&url).await
            .context("PCGW search request failed")?;

        let results = body
            .pointer("/query/search")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut out = Vec::with_capacity(results.len());

        for entry in results {
            let title = match entry.get("title").and_then(Value::as_str) {
                Some(t) => t.to_string(),
                None => continue,
            };
            let pageid = match entry.get("pageid").and_then(Value::as_u64) {
                Some(id) => id,
                None => continue,
            };

            // Use parse only for title/pageid resolution (follows redirects).
            let parse_data = match self.parse_page(&title, fetch_mode).await {
                Ok(v) => v,
                Err(e) => {
                    debug!(error = %e, title, "PCGW parse failed, falling back to search entry");
                    entry.clone()
                }
            };

            let resolved_title = parse_data
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&title)
                .to_string();

            let resolved_id = parse_data
                .get("pageid")
                .and_then(Value::as_u64)
                .unwrap_or(pageid)
                .to_string();

            // Fetch structured Cargo data sequentially (rate-limited).
            let cargo_data = match self.cargo_query_page(&resolved_title, fetch_mode).await {
                Ok(v) => v,
                Err(e) => {
                    debug!(error = %e, title = resolved_title, "PCGW Cargo query failed");
                    Value::Object(Map::new())
                }
            };

            let mut data = Map::new();
            data.insert("parse".to_string(), parse_data);
            data.insert("cargo".to_string(), cargo_data);

            let description = entry
                .get("snippet")
                .and_then(Value::as_str)
                .map(|s| ammonia::clean_text(s));

            out.push(SearchResult {
                provider: "pcgw".to_string(),
                id: resolved_id,
                label: resolved_title,
                description,
                item_type: Some("video-game".to_string()),
                data: Value::Object(data),
            });
        }

        // Cache the assembled result list.
        if let Ok(json) = serde_json::to_string(&out) {
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(out)
    }
}

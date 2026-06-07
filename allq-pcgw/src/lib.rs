use allq_core::{FetchMode, GameStoreType, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use anyhow::Context;
use async_trait::async_trait;
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use reqwest::Url;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, error, trace, warn};

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
    #[tracing::instrument(skip(self))]
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

        debug!(%url);
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

    async fn rate_limited_get_url(&self, url: &str) -> anyhow::Result<Url> {
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

        Ok(self.client
            .get(url)
            .send()
            .await
            .context("PCGW request failed")?
            .error_for_status()
            .context("PCGW returned error status")?
            .url()
            .clone())
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

    async fn get_title_and_id_from_search_query(&self, query: &str, limit: u32) -> anyhow::Result<Vec<Value>> {
        let url = format!(
            "https://www.pcgamingwiki.com/w/api.php\
             ?action=query&list=search&srsearch={}&srlimit={}&format=json",
            urlencoding::encode(query),
            limit,
        );

        debug!(%url, "searching PCGW");

        let body: Value = self.rate_limited_get(&url).await
            .context("PCGW search request failed")?;

        // this parses search results and then searches based on title
        let results = body
            .pointer("/query/search")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(results)
    }

    async fn process_search_entries(&self, fetch_mode: FetchMode, results: Vec<Value>, out: &mut Vec<SearchResult>) -> anyhow::Result<()> {
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
            // TODO: evaluate whether we can/should hoist "dynamic" data out of the "only if not returning cached data" flow
            //   some values are not accessibly presented as properties
            //   some examples of these values include those under "Game data"
            //   This list consists of:
            //   - Configuration file(s) location
            //   - Save game data location
            //   - Save game cloud syncing
            //  It might be better if we exclude this data from the initial search results,
            //  and instead treat them as post-process enrichments
            //  That is to say, perhaps we should "recalculate" these values
            //  even when returning cached results
            //  If there is a noticable impact on latency, then perhaps another flag makes sense
            //  For example, "recalculate enrichments" that is distinct from "force fetch",
            //  "cached only", and "direct only"
            let save_game_locations = &mut get_save_game_and_config_locations(&parse_data, &cargo_data)?;
            data.insert("parse".to_string(), parse_data);
            data.insert("cargo".to_string(), cargo_data);
            data.append(save_game_locations);

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
        Ok(())
    }

    async fn process_data_entry(&self, fetch_mode: FetchMode, parse_data: Value) -> anyhow::Result<SearchResult> {

        let parsed = parse_data.clone();
        let parsed = parsed.get("parse")
            .context("Failed to get parse text")?
            .clone();

        trace!(?parsed);
        let parsed = parsed.clone();
        let resolved_title = parsed.clone();
        let resolved_title = resolved_title.get("title").clone();
        let resolved_title = resolved_title.and_then(Value::as_str).clone();
        let resolved_title = resolved_title.as_ref().clone().unwrap_or(&"");
        debug!(?resolved_title);
        let resolved_pageid = parsed.get("pageid").and_then(Value::as_u64).unwrap_or_default();
        debug!(?resolved_pageid);

        // TODO: can/should we do this based on PageID instead?
        // Fetch structured Cargo data sequentially (rate-limited).
        let cargo_data = match self.cargo_query_page(&resolved_title, fetch_mode).await {
            Ok(v) => v,
            Err(e) => {
                debug!(error = %e, title = resolved_title, "PCGW Cargo query failed");
                Value::Object(Map::new())
            }
        };

        let mut data = Map::new();
        let save_game_locations = &mut get_save_game_and_config_locations(&parsed, &cargo_data)?;
        data.insert("parse".to_string(), parsed);
        data.insert("cargo".to_string(), cargo_data);
        data.append(save_game_locations);

        let description = "N/A";

        Ok(SearchResult {
            provider: "pcgw".to_string(),
            id: resolved_pageid.to_string(),
            label: resolved_title.to_string(),
            description: Some(description.to_string()),
            item_type: Some("video-game".to_string()),
            data: Value::Object(data),
        })
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

        debug!(?options);

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

        let game_entries = match &options.provider_direct_id_search {
            None => {
                let results = self.get_title_and_id_from_search_query(query, limit).await?;
                let mut out = Vec::with_capacity(results.len());

                self.process_search_entries(fetch_mode, results, &mut out).await?;
                out
            }
            Some(t) => {
                let game_page_url = get_url(self, query, limit, options).await?;
                let data = self.rate_limited_get(&game_page_url.clone()).await?;
                vec![self.process_data_entry(fetch_mode, data).await?]
            }
        };

        // Cache the assembled result list.
        if let Ok(json) = serde_json::to_string(&game_entries) {
            self.cache_insert(search_cache_key, json, fetch_mode).await;
        }

        Ok(game_entries)
    }
}

#[tracing::instrument(skip(provider))]
async fn get_url(
    provider: &PcgwSearchProvider,
    query: &str,
    limit: u32,
    search_options: &SearchOptions,
) -> anyhow::Result<String> {
    let default_url = format!(
        "https://www.pcgamingwiki.com/w/api.php\
             ?action=query&list=search&srsearch={}&srlimit={}&format=json",
        urlencoding::encode(query),
        limit,
    );
    match search_options.provider_direct_id_search.as_ref() {
        None => {
            debug!(%query, %default_url, "Searching by title");
            warn!("Defaulting!");
            Ok(default_url)
        }
        Some(game_store) => {
            let redirect_url = match game_store {
                GameStoreType::Steam => {
                    debug!(%query, "Searching by Steam App ID");
                    format!(
                        "https://www.pcgamingwiki.com/w/api.php\
                        ?action=cargoquery\
                        &tables=Infobox_game\
                        &fields=Infobox_game._pageID%3DPageID%2CInfobox_game.Steam_AppID\
                        &where=Infobox_game.Steam_AppID%20HOLDS%20%22{query}%22\
                        &format=json"
                    )
                }
                GameStoreType::Gog => {
                    debug!(%query, "Searching by GOG ID");
                    format!(
                        "https://www.pcgamingwiki.com/w/api.php\
                        ?action=cargoquery\
                        &tables=Infobox_game\
                        &fields=Infobox_game._pageID%3DPageID%2CInfobox_game.GOGcom_ID\
                        &where=Infobox_game.GOGcom_ID%20HOLDS%20%22{query}%22\
                        &format=json"
                    )
                }
            };
            let redirect_url = provider.rate_limited_get(redirect_url.as_str())
                .await
                .inspect_err(|error| {
                    error!(?error);
                })
                .context("Failed to resolve App ID redirect JSON")?;
            let Some(root_object) = redirect_url.as_object() else { return Ok(default_url) };
            let Some(cargoquery) = root_object.get("cargoquery") else { return Ok(default_url) };
            debug!(%cargoquery);
            let Some(titles) = cargoquery.as_array() else { return Ok(default_url) };
            let Some(first_result) = titles.first() else { return Ok(default_url) };
            let Some(title) = first_result.get("title") else { return Ok(default_url) };
            let Some(page_id_value) = title.get("PageID") else { return Ok(default_url) };
            let Some(page_id) = page_id_value.as_str() else { return Ok(default_url) };
            Ok(format!(
                "https://www.pcgamingwiki.com/w/api.php\
        ?action=parse&format=json&pageid={}&format=json",
                urlencoding::encode(page_id),
            ))
        }
    }
}

fn get_save_game_and_config_locations(parse_data: &Value, cargo_data: &Value) -> anyhow::Result<Map<String, Value>> {
    debug!(
        ?parse_data,
        ?cargo_data,
    );
    let data = parse_data.get("text")
        .context("Failed to unwrap parse data text")?
        .as_object()
        .context("Failed to get text as object")?
        .get("*")
        .context("Failed to get * string from text object")?
        .as_str()
        .context("Failed to get string value from * from text object")?;

    // need to grab "Configuration file(s) location", "Save game data location", and "Save Game Cloud Syncing" tables
    // xpath: //h3/span[contains(text(), 'Configuration file(s) location')]/following::div[1][contains(@class, 'container-pcgwikitable')]/table/tbody/tr/text()[1]
    // xpath: //h3/span[contains(text(), 'Save game data location')]/following::div[1][contains(@class, 'container-pcgwikitable')]/table/tbody/tr/text()[1]
    let save_game_location_and_configuration_data = get_save_game_location_and_configuration_data(&data)
        .unwrap_or(Value::Null);

    // xpath: //h3/span[contains(text(), 'Save game cloud syncing')]/following::div[1][contains(@class, 'container-pcgwikitable')]/table/tbody/tr/text()[1]
    let save_game_cloud_syncing_data = get_save_game_cloud_syncing_data(&data)
        .unwrap_or(Value::Null);
    let mut map = Map::new();

    let mut game_map = Map::new();
    &mut append_nested_json_map_if_exists(
        &save_game_location_and_configuration_data,
        "Configuration file(s) location",
        "configuration_files_location",
        &mut game_map
    )?;

    &mut append_nested_json_map_if_exists(
        &save_game_location_and_configuration_data,
        "Save game data location",
        "save_game_data_location",
        &mut game_map
    )?;

    &mut append_nested_json_map_if_exists(
        &save_game_cloud_syncing_data,
        "Save game cloud syncing",
        "save_game_cloud_syncing",
        &mut game_map
    )?;

    map.insert("game_data".to_string(), Value::Object(game_map));

    Ok(map)
}

#[tracing::instrument(skip(data))]
fn append_nested_json_map_if_exists(
    data: &Value,
    path: &str,
    name: &str,
    out_map: &mut Map<String, Value>
) -> anyhow::Result<()> {
    match data.get(path) {
        None => {
            // Value doesn't exist
            // Don't append anything but return `Ok(())` / Success
            debug!(
                ?path,
                ?data,
                "No value found under path"
            );
            Ok(())
        }
        Some(map) => {
            debug!(
                ?path,
                ?data,
                ?map,
                "Found value under path"
            );

            out_map.insert(name.to_string(), map.clone());
            Ok(())
        }
    }
}

fn extract_pcgw_cell_value(cell: ElementRef<'_>, title_selector: &Selector) -> String {
    let raw_text = cell
        .text()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let normalized_text = normalize_pcgw_cell_text(&raw_text);
    if !normalized_text.is_empty() {
        return normalized_text;
    }

    cell.value()
        .attr("title")
        .or_else(|| {
            cell.select(title_selector)
                .find_map(|element| element.value().attr("title"))
        })
        .map(normalize_pcgw_cell_text)
        .unwrap_or_default()
}

fn get_save_game_location_and_configuration_data(data: &str) -> anyhow::Result<Value> {
    // headers (tr): System\tLocation
    // possible rows (th): Windows, Mac OS (Classic), macOS (OS X), Steam Play (Linux) || Linux
    extract_pcgw_tables(
        data,
        &[
            "Configuration file(s) location",
            "Save game data location",
        ],
    )
}

fn get_save_game_cloud_syncing_data(data: &str) -> anyhow::Result<Value> {
    // NOTE: this data is also provided by Cargo
    //  See: cargo > Cloud
    //  Example output:
    //  {
    //    "Steam": "false",
    //    "GOG Galaxy": null,
    //    "Epic Games Launcher": null,
    //    "Xbox": null
    //  }
    //  However, this data does not include notes for the associated system type

    extract_pcgw_tables(data, &["Save game cloud syncing"])
}

fn normalize_pcgw_cell_text(raw: &str) -> String {
    let collapsed = collapse_pcgw_cell_whitespace(raw);
    strip_pcgw_note_markers(&collapsed)
}

fn collapse_pcgw_cell_whitespace(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            while matches!(chars.peek().copied(), Some(next) if next.is_whitespace()) {
                chars.next();
            }

            let previous_is_path_separator = output
                .chars()
                .next_back()
                .is_some_and(|previous| matches!(previous, '/' | '\\'));
            let next_is_path_separator = matches!(chars.peek().copied(), Some('/') | Some('\\'));

            if !previous_is_path_separator && !next_is_path_separator {
                output.push(' ');
            }
        } else {
            output.push(ch);
        }
    }

    output.trim().to_string()
}

fn strip_pcgw_note_markers(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let bytes = text.as_bytes();

    while index < text.len() {
        if bytes[index..].starts_with(b"[Note ") {
            let mut cursor = index + b"[Note ".len();
            let digit_start = cursor;

            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }

            if cursor > digit_start && cursor < bytes.len() && bytes[cursor] == b']' {
                index = cursor + 1;
                continue;
            }
        }

        let ch = text[index..]
            .chars()
            .next()
            .expect("index is always on a UTF-8 character boundary");
        output.push(ch);
        index += ch.len_utf8();
    }

    collapse_pcgw_cell_whitespace(&output)
}

fn extract_pcgw_tables(data: &str, headings: &[&str]) -> anyhow::Result<Value> {
    let document = Html::parse_document(data);

    let h3_selector = Selector::parse("h3").unwrap();
    let table_container_selector = Selector::parse("div.container-pcgwikitable").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("th, td").unwrap();
    let title_selector = Selector::parse("[title]").unwrap();

    let mut output = Map::new();

    for heading in headings {
        let Some(h3) = document
            .select(&h3_selector)
            .find(|h3| h3.text().collect::<String>().contains(heading))
        else {
            continue;
        };

        let Some(container) = h3
            .next_siblings()
            .filter_map(ElementRef::wrap)
            .find(|element| {
                element
                    .select(&table_container_selector)
                    .next()
                    .is_some()
                    || element.value().has_class(
                    "container-pcgwikitable",
                    scraper::CaseSensitivity::AsciiCaseInsensitive,
                )
            })
        else {
            continue;
        };

        let table_container = if container
            .value()
            .has_class("container-pcgwikitable", scraper::CaseSensitivity::AsciiCaseInsensitive)
        {
            container
        } else {
            container
                .select(&table_container_selector)
                .next()
                .context("found wrapper but no PCGW table container")?
        };

        let rows = table_container
            .select(&row_selector)
            .map(|row| {
                row.select(&cell_selector)
                    .map(|cell| extract_pcgw_cell_value(cell, &title_selector))
                    .collect::<Vec<_>>()
            })
            .filter(|row| row.iter().any(|cell| !cell.is_empty()))
            .collect::<Vec<_>>();

        let Some((headers, body_rows)) = rows.split_first() else {
            continue;
        };

        let structured_rows = body_rows
            .iter()
            .map(|cells| {
                let values = cells
                    .iter()
                    .enumerate()
                    .map(|(index, cell)| {
                        let key = headers
                            .get(index)
                            .filter(|header| !header.is_empty())
                            .cloned()
                            .unwrap_or_else(|| format!("column_{index}"));

                        (key, Value::String(cell.clone()))
                    })
                    .collect::<Map<_, _>>();

                Value::Object(values)
            })
            .collect::<Vec<_>>();

        output.insert((*heading).to_string(), Value::Array(structured_rows));
    }

    if output.is_empty() {
        anyhow::bail!("No matching PCGW tables found for headings: {headings:?}");
    }

    Ok(Value::Object(output))
}

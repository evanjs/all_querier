use std::collections::HashMap;
use std::time::Instant;

use anyhow::Context;
use mediawiki::prelude::*;
use serde_json::Value;
use tokio::sync::OnceCell;
use tracing::debug;

use crate::cache::{
    WikidataCache, create_wikidata_cache,
};

pub const WIKIDATA_API_URL: &str = "https://www.wikidata.org/w/api.php";

pub const ENTITY_QUERY_PROPS: &str = "info|labels|descriptions|aliases|claims|sitelinks";

const WIKIMEDIA_ACCESS_TOKEN_ENV: &str = "WIKIMEDIA_ACCESS_TOKEN";
const WIKIDATA_MAXLAG_ENV: &str = "ALLQ_WIKIDATA_MAXLAG";
const DEFAULT_LANGUAGE: &str = "en";
const DEFAULT_FORMAT: &str = "json";
const DEFAULT_FORMAT_VERSION: &str = "2";
const DEFAULT_MAXLAG: &str = "5";


pub struct WikidataClient {
    api: Option<OnceCell<Api>>,
    cache: Option<WikidataCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WikidataEntityLookupMode {
    NetworkFallback,
    CacheOnly,
    ForceFetch,
}

impl WikidataClient {
    pub async fn new() -> anyhow::Result<Self> {
        let cache = create_wikidata_cache().await?;

        Ok(Self {
            api: Some(OnceCell::new()),
            cache: Some(cache),
        })
    }

    pub async fn new_local_only() -> anyhow::Result<Self> {
        let cache = create_wikidata_cache().await?;

        Ok(Self {
            api: None,
            cache: Some(cache),
        })
    }

    pub fn from_api(api: Api) -> Self {
        let api_cell = OnceCell::new();
        let _ = api_cell.set(api);

        Self {
            api: Some(api_cell),
            cache: None,
        }
    }

    pub async fn api(&self) -> anyhow::Result<&Api> {
        let api = self
            .api
            .as_ref()
            .context("Wikidata API is unavailable in local-only cache lookup mode")?;

        let started_at = Instant::now();
        let api = api.get_or_try_init(wikidata_api).await?;

        debug!(
            elapsed = ?started_at.elapsed(),
            "Wikidata API client ready",
        );

        Ok(api)
    }

    pub async fn userinfo(&self) -> anyhow::Result<Value> {
        let params = query_params(&[
            ("action", "query"),
            ("meta", "userinfo"),
            ("format", DEFAULT_FORMAT),
        ]);

        self.query_api_json(&params).await
    }

    pub async fn entity_by_qid(&self, qid: &str) -> anyhow::Result<Value> {
        self.entity_by_qid_with_mode(qid, WikidataEntityLookupMode::NetworkFallback)
            .await
    }

    pub async fn entity_by_qid_with_mode(
        &self,
        qid: &str,
        lookup_mode: WikidataEntityLookupMode,
    ) -> anyhow::Result<Value> {
        let started_at = Instant::now();
        let qid = normalize_qid(qid)?;
        let cache_key = wikidata_entity_cache_key(qid);

        debug!(
            qid,
            cache_key,
            lookup_mode = ?lookup_mode,
            "entity_by_qid started",
        );

        if lookup_mode != WikidataEntityLookupMode::ForceFetch {
            if let Some(cache) = &self.cache {
                let cache_lookup_started_at = Instant::now();

                if let Some(entry) = cache.get(&cache_key).await? {
                    let cache_lookup_elapsed = cache_lookup_started_at.elapsed();
                    let s = entry.value().clone();
                    let bytes = s.len();

                    let parse_started_at = Instant::now();
                    let parsed = serde_json::from_str::<Value>(&s)?;

                    debug!(
                        qid,
                        cache_key,
                        bytes,
                        cache_lookup_elapsed = ?cache_lookup_elapsed,
                        parse_elapsed = ?parse_started_at.elapsed(),
                        total_elapsed = ?started_at.elapsed(),
                        "entity_by_qid cache hit",
                    );

                    return Ok(parsed);
                }

                debug!(
                    qid,
                    cache_key,
                    cache_lookup_elapsed = ?cache_lookup_started_at.elapsed(),
                    "entity_by_qid cache miss",
                );
            } else {
                debug!(
                    qid,
                    "entity_by_qid has no cache configured",
                );
            }
        } else {
            debug!(
                qid,
                "entity_by_qid force-fetch enabled; skipping cache read",
            );
        }

        if lookup_mode == WikidataEntityLookupMode::CacheOnly {
            anyhow::bail!("Wikidata entity {qid} was not found in the local cache");
        }

        let params = wbgetentities_params(qid);

        debug!(
            qid,
            props = ENTITY_QUERY_PROPS,
            maxlag = params
                .get("maxlag")
                .map(String::as_str)
                .unwrap_or("<disabled>"),
            "entity_by_qid fetching from Wikidata",
        );

        let fetch_started_at = Instant::now();
        let res = self.query_api_json(&params).await?;
        let fetch_elapsed = fetch_started_at.elapsed();

        let response_bytes = serde_json::to_string(&res)
            .map(|s| s.len())
            .unwrap_or_default();

        debug!(
            qid,
            response_bytes,
            fetch_elapsed = ?fetch_elapsed,
            has_error = res.get("error").is_some(),
            "entity_by_qid fetch completed",
        );

        if should_cache_wikidata_response(&res) {
            if let Some(cache) = &self.cache {
                let cache_write_started_at = Instant::now();
                let s = serde_json::to_string(&res)?;
                let bytes = s.len();

                cache.insert(cache_key.clone(), s);

                debug!(
                    qid,
                    cache_key,
                    bytes,
                    cache_write_elapsed = ?cache_write_started_at.elapsed(),
                    "entity_by_qid stored in cache",
                );
            }
        } else {
            debug!(
                qid,
                error = ?res.get("error"),
                "entity_by_qid response not cached because it contains error",
            );
        }

        debug!(
            qid,
            total_elapsed = ?started_at.elapsed(),
            "entity_by_qid completed",
        );

        Ok(res)
    }

    pub async fn entities_by_qids_with_mode(
        &self,
        qids: &[String],
        lookup_mode: WikidataEntityLookupMode,
    ) -> anyhow::Result<Value> {
        let started_at = Instant::now();

        let mut qids = qids
            .iter()
            .map(|qid| normalize_qid(qid).map(ToString::to_string))
            .collect::<anyhow::Result<Vec<_>>>()?;

        qids.sort();
        qids.dedup();

        debug!(
            qid_count = qids.len(),
            qids = ?qids,
            lookup_mode = ?lookup_mode,
            "entities_by_qids started",
        );

        let mut entities = serde_json::Map::new();
        let mut missing_qids = Vec::new();

        if lookup_mode != WikidataEntityLookupMode::ForceFetch {
            if let Some(cache) = &self.cache {
                let all_cache_lookup_started_at = Instant::now();

                for qid in &qids {
                    let cache_key = wikidata_entity_cache_key(qid);
                    let cache_lookup_started_at = Instant::now();

                    if let Some(entry) = cache.get(&cache_key).await? {
                        let cache_lookup_elapsed = cache_lookup_started_at.elapsed();
                        let s = entry.value().clone();
                        let bytes = s.len();

                        let parse_started_at = Instant::now();
                        let parsed = serde_json::from_str::<Value>(&s)?;
                        let parse_elapsed = parse_started_at.elapsed();

                        if let Some(entity) = parsed
                            .get("entities")
                            .and_then(|entities| entities.get(qid))
                            .cloned()
                        {
                            entities.insert(qid.clone(), entity);

                            debug!(
                                qid,
                                cache_key,
                                bytes,
                                cache_lookup_elapsed = ?cache_lookup_elapsed,
                                parse_elapsed = ?parse_elapsed,
                                "entities_by_qids entity cache hit",
                            );

                            continue;
                        }

                        debug!(
                            qid,
                            cache_key,
                            bytes,
                            cache_lookup_elapsed = ?cache_lookup_elapsed,
                            parse_elapsed = ?parse_elapsed,
                            "entities_by_qids cache entry did not contain requested entity",
                        );
                    } else {
                        debug!(
                            qid,
                            cache_key,
                            cache_lookup_elapsed = ?cache_lookup_started_at.elapsed(),
                            "entities_by_qids entity cache miss",
                        );
                    }

                    missing_qids.push(qid.clone());
                }

                debug!(
                    requested_qid_count = qids.len(),
                    cache_hit_count = entities.len(),
                    cache_miss_count = missing_qids.len(),
                    cache_lookup_elapsed = ?all_cache_lookup_started_at.elapsed(),
                    "entities_by_qids cache lookup completed",
                );
            } else {
                debug!(
                    qid_count = qids.len(),
                    "entities_by_qids has no cache configured",
                );

                missing_qids.extend(qids.iter().cloned());
            }
        } else {
            debug!(
                qid_count = qids.len(),
                "entities_by_qids force-fetch enabled; skipping cache read",
            );

            missing_qids.extend(qids.iter().cloned());
        }

        if lookup_mode == WikidataEntityLookupMode::CacheOnly && !missing_qids.is_empty() {
            anyhow::bail!(
                    "Wikidata entities were not found in the local cache: {}",
                    missing_qids.join(", "),
                );
        }

        if !missing_qids.is_empty() {
            let ids = missing_qids.join("|");
            let params = wbgetentities_params(&ids);

            debug!(
                    missing_qid_count = missing_qids.len(),
                    missing_qids = ?missing_qids,
                    props = ENTITY_QUERY_PROPS,
                    maxlag = params
                        .get("maxlag")
                        .map(String::as_str)
                        .unwrap_or("<disabled>"),
                    "entities_by_qids fetching missing entities from Wikidata",
                );

            let fetch_started_at = Instant::now();
            let res = self.query_api_json(&params).await?;
            let fetch_elapsed = fetch_started_at.elapsed();

            let response_bytes = serde_json::to_string(&res)
                .map(|s| s.len())
                .unwrap_or_default();

            debug!(
                missing_qid_count = missing_qids.len(),
                response_bytes,
                fetch_elapsed = ?fetch_elapsed,
                has_error = res.get("error").is_some(),
                error = ?res.get("error"),
                "entities_by_qids fetch completed",
            );

            let fetched_entities = res
                .get("entities")
                .and_then(Value::as_object)
                .context("Wikidata entity response is missing entities object")?;

            let should_cache = should_cache_wikidata_response(&res);
            let cache_write_started_at = Instant::now();
            let mut cached_count = 0usize;
            let mut cached_bytes = 0usize;

            for qid in &missing_qids {
                let entity = fetched_entities
                    .get(qid)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Wikidata entity response did not include {qid}"))?;

                entities.insert(qid.clone(), entity.clone());

                if should_cache {
                    if let Some(cache) = &self.cache {
                        let mut single_entities = serde_json::Map::new();
                        single_entities.insert(qid.clone(), entity);

                        let single_entity_response = serde_json::json!({
                                "entities": single_entities,
                            });
                        let cache_key = wikidata_entity_cache_key(qid);
                        let s = serde_json::to_string(&single_entity_response)?;

                        cached_bytes += s.len();
                        cache.insert(cache_key.clone(), s);
                        cached_count += 1;

                        debug!(
                            qid,
                            cache_key,
                            "entities_by_qids stored entity in cache",
                        );
                    }
                }
            }

            if should_cache {
                debug!(
                    cached_count,
                    cached_bytes,
                    cache_write_elapsed = ?cache_write_started_at.elapsed(),
                    "entities_by_qids cache writes completed",
                );
            } else {
                debug!(
                    error = ?res.get("error"),
                    "entities_by_qids response not cached because it contains error",
                );
            }
        }

        debug!(
            returned_entity_count = entities.len(),
            total_elapsed = ?started_at.elapsed(),
            "entities_by_qids completed",
        );

        Ok(serde_json::json!({
                "entities": entities,
            }))
    }

    pub async fn query_api_json(&self, params: &HashMap<String, String>) -> anyhow::Result<Value> {
        let action = params
            .get("action")
            .map(String::as_str)
            .unwrap_or("<missing>");
        let started_at = Instant::now();

        debug!(
            action,
            params = ?params,
            "Wikidata API request started",
        );

        let api = self.api().await?;
        let res = match api.get_query_api_json(params).await {
            Ok(res) => res,
            Err(error) => {
                debug!(
                    action,
                    elapsed = ?started_at.elapsed(),
                    error = %error,
                    "Wikidata API request failed",
                );

                return Err(error.into());
            }
        };

        debug!(
            action,
            elapsed = ?started_at.elapsed(),
            has_error = res.get("error").is_some(),
            error = ?res.get("error"),
            "Wikidata API request completed",
        );

        Ok(res)
    }

    pub async fn sparql_query_json(&self, query: &str) -> anyhow::Result<Value> {
        let started_at = Instant::now();

        debug!(
            query = %query,
            "Wikidata SPARQL request started",
        );

        let res = self.api().await?.sparql_query(query).await?;

        let binding_count = res
            .pointer("/results/bindings")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let response_bytes = serde_json::to_string(&res)
            .map(|s| s.len())
            .unwrap_or_default();

        debug!(
            elapsed = ?started_at.elapsed(),
            binding_count,
            response_bytes,
            has_error = res.get("error").is_some(),
            error = ?res.get("error"),
            "Wikidata SPARQL request completed",
        );

        Ok(res)
    }

    pub fn cache_as_ref(&self) -> Option<&WikidataCache> {
        self.cache.as_ref()
    }
}

pub async fn wikidata_api() -> anyhow::Result<Api> {
    let started_at = Instant::now();

    debug!(
        url = WIKIDATA_API_URL,
        "creating Wikidata API client",
    );

    let api_new_started_at = Instant::now();
    let mut api = Api::new(WIKIDATA_API_URL).await?;

    debug!(
        elapsed = ?api_new_started_at.elapsed(),
        total_elapsed = ?started_at.elapsed(),
        "Wikidata Api::new completed",
    );

    if let Some(access_token) = wikimedia_access_token() {
        let oauth_started_at = Instant::now();

        api.set_oauth2(&access_token);

        debug!(
            elapsed = ?oauth_started_at.elapsed(),
            total_elapsed = ?started_at.elapsed(),
            "Wikidata API OAuth token configured",
        );
    } else {
        debug!(
            total_elapsed = ?started_at.elapsed(),
            "Wikidata API OAuth token not configured",
        );
    }

    debug!(
        total_elapsed = ?started_at.elapsed(),
        "created Wikidata API client",
    );

    Ok(api)
}

fn query_params(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn wbgetentities_params(ids: &str) -> HashMap<String, String> {
    let mut params = query_params(&[
        ("action", "wbgetentities"),
        ("ids", ids),
        ("props", ENTITY_QUERY_PROPS),
        ("languages", DEFAULT_LANGUAGE),
        ("format", DEFAULT_FORMAT),
        ("formatversion", DEFAULT_FORMAT_VERSION),
    ]);

    if let Some(maxlag) = wikidata_maxlag() {
        params.insert("maxlag".to_string(), maxlag);
    }

    params
}

fn wikidata_maxlag() -> Option<String> {
    // Interactive tasks (where a user is waiting for the result) may omit the maxlag parameter.
    // Noninteractive tasks should always use it.
    // See also API:Etiquette#The maxlag parameter.
    // https://www.mediawiki.org/wiki/Special:MyLanguage/API:Etiquette#The_maxlag_parameter
    let Ok(value) = std::env::var(WIKIDATA_MAXLAG_ENV) else {
        return None;
    };

    let value = value.trim();

    if value.is_empty()
        || value.eq_ignore_ascii_case("off")
        || value.eq_ignore_ascii_case("none")
        || value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("disabled")
    {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_qid(qid: &str) -> anyhow::Result<&str> {
    let qid = qid.trim();

    anyhow::ensure!(!qid.is_empty(), "Wikidata QID cannot be empty");

    Ok(qid)
}

fn wikidata_entity_cache_key(qid: &str) -> String {
    format!("entity_by_qid:{qid}")
}

fn should_cache_wikidata_response(value: &Value) -> bool {
    value.get("error").is_none()
}

fn wikimedia_access_token() -> Option<String> {
    std::env::var(WIKIMEDIA_ACCESS_TOKEN_ENV)
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}
use std::collections::HashMap;

use anyhow::Context;
use mediawiki::prelude::*;
use serde_json::Value;

use crate::cache::{
    WikidataCache, create_wikidata_cache,
};

pub const WIKIDATA_API_URL: &str = "https://www.wikidata.org/w/api.php";

pub const ENTITY_QUERY_PROPS: &str = "info|labels|descriptions|aliases|claims|sitelinks";

const WIKIMEDIA_ACCESS_TOKEN_ENV: &str = "WIKIMEDIA_ACCESS_TOKEN";
const DEFAULT_LANGUAGE: &str = "en";
const DEFAULT_FORMAT: &str = "json";
const DEFAULT_FORMAT_VERSION: &str = "2";
const DEFAULT_MAXLAG: &str = "5";

pub struct WikidataClient {
    api: Option<Api>,
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
        let api = wikidata_api().await?;
        let cache = create_wikidata_cache().await?;

        Ok(Self {
            api: Some(api),
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
        Self {
            api: Some(api),
            cache: None,
        }
    }

    pub fn api(&self) -> anyhow::Result<&Api> {
        self.api
            .as_ref()
            .context("Wikidata API is unavailable in local-only cache lookup mode")
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
        let qid = normalize_qid(qid)?;
        let cache_key = wikidata_entity_cache_key(qid);

        if lookup_mode != WikidataEntityLookupMode::ForceFetch {
            if let Some(cache) = &self.cache {
                if let Some(entry) = cache.get(&cache_key).await? {
                    let s = entry.value().clone();
                    let parsed = serde_json::from_str::<Value>(&s)?;
                    return Ok(parsed);
                }
            }
        }

        if lookup_mode == WikidataEntityLookupMode::CacheOnly {
            anyhow::bail!("Wikidata entity {qid} was not found in the local cache");
        }

        let params = query_params(&[
            ("action", "wbgetentities"),
            ("ids", qid),
            ("props", ENTITY_QUERY_PROPS),
            ("languages", DEFAULT_LANGUAGE),
            ("format", DEFAULT_FORMAT),
            ("formatversion", DEFAULT_FORMAT_VERSION),
            ("maxlag", DEFAULT_MAXLAG),
        ]);

        let res = self.query_api_json(&params).await?;

        if should_cache_wikidata_response(&res) {
            if let Some(cache) = &self.cache {
                let s = serde_json::to_string(&res)?;
                cache.insert(cache_key, s);
            }
        }

        Ok(res)
    }

    pub async fn entities_by_qids_with_mode(
        &self,
        qids: &[String],
        lookup_mode: WikidataEntityLookupMode,
    ) -> anyhow::Result<Value> {
        let mut qids = qids
            .iter()
            .map(|qid| normalize_qid(qid).map(ToString::to_string))
            .collect::<anyhow::Result<Vec<_>>>()?;

        qids.sort();
        qids.dedup();

        let mut entities = serde_json::Map::new();
        let mut missing_qids = Vec::new();

        if lookup_mode != WikidataEntityLookupMode::ForceFetch {
            if let Some(cache) = &self.cache {
                for qid in &qids {
                    let cache_key = wikidata_entity_cache_key(qid);

                    if let Some(entry) = cache.get(&cache_key).await? {
                        let s = entry.value().clone();
                        let parsed = serde_json::from_str::<Value>(&s)?;

                        if let Some(entity) = parsed
                            .get("entities")
                            .and_then(|entities| entities.get(qid))
                            .cloned()
                        {
                            entities.insert(qid.clone(), entity);
                            continue;
                        }
                    }

                    missing_qids.push(qid.clone());
                }
            } else {
                missing_qids.extend(qids.iter().cloned());
            }
        } else {
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
            let params = query_params(&[
                ("action", "wbgetentities"),
                ("ids", &ids),
                ("props", ENTITY_QUERY_PROPS),
                ("languages", DEFAULT_LANGUAGE),
                ("format", DEFAULT_FORMAT),
                ("formatversion", DEFAULT_FORMAT_VERSION),
                ("maxlag", DEFAULT_MAXLAG),
            ]);

            let res = self.query_api_json(&params).await?;
            let fetched_entities = res
                .get("entities")
                .and_then(Value::as_object)
                .context("Wikidata entity response is missing entities object")?;

            for qid in &missing_qids {
                let entity = fetched_entities
                    .get(qid)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Wikidata entity response did not include {qid}"))?;

                entities.insert(qid.clone(), entity.clone());

                if should_cache_wikidata_response(&res) {
                    if let Some(cache) = &self.cache {
                        let mut single_entities = serde_json::Map::new();
                        single_entities.insert(qid.clone(), entity);

                        let single_entity_response = serde_json::json!({
                                "entities": single_entities,
                            });
                        let cache_key = wikidata_entity_cache_key(qid);
                        let s = serde_json::to_string(&single_entity_response)?;

                        cache.insert(cache_key, s);
                    }
                }
            }
        }

        Ok(serde_json::json!({
                "entities": entities,
            }))
    }

    pub async fn query_api_json(&self, params: &HashMap<String, String>) -> anyhow::Result<Value> {
        let api = self
            .api
            .as_ref()
            .context("Wikidata API is unavailable in local-only cache lookup mode")?;

        Ok(api.get_query_api_json(params).await?)
    }

    pub async fn sparql_query_json(&self, query: &str) -> anyhow::Result<Value> {
        Ok(self.api()?.sparql_query(query).await?)
    }

    pub fn cache_as_ref(&self) -> Option<&WikidataCache> {
        self.cache.as_ref()
    }
}

pub async fn wikidata_api() -> anyhow::Result<Api> {
    let mut api = Api::new(WIKIDATA_API_URL).await?;

    if let Some(access_token) = wikimedia_access_token() {
        api.set_oauth2(&access_token);
    }

    Ok(api)
}

fn query_params(entries: &[(&str, &str)]) -> HashMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
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
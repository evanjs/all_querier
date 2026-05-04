use std::collections::HashMap;

use mediawiki::prelude::*;
use serde_json::Value;

pub const WIKIDATA_API_URL: &str = "https://www.wikidata.org/w/api.php";

pub const ENTITY_QUERY_PROPS: &str = "info|labels|descriptions|aliases|claims|sitelinks";

const WIKIMEDIA_ACCESS_TOKEN_ENV: &str = "WIKIMEDIA_ACCESS_TOKEN";
const DEFAULT_LANGUAGE: &str = "en";
const DEFAULT_FORMAT: &str = "json";
const DEFAULT_FORMAT_VERSION: &str = "2";
const DEFAULT_MAXLAG: &str = "5";

pub struct WikidataClient {
    api: Api,
}

impl WikidataClient {
    pub async fn new() -> anyhow::Result<Self> {
        let api = wikidata_api().await?;

        Ok(Self { api })
    }

    pub fn from_api(api: Api) -> Self {
        Self { api }
    }

    pub fn api(&self) -> &Api {
        &self.api
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
        let qid = normalize_qid(qid)?;

        let params = query_params(&[
            ("action", "wbgetentities"),
            ("ids", qid),
            ("props", ENTITY_QUERY_PROPS),
            ("languages", DEFAULT_LANGUAGE),
            ("format", DEFAULT_FORMAT),
            ("formatversion", DEFAULT_FORMAT_VERSION),
            ("maxlag", DEFAULT_MAXLAG),
        ]);

        self.query_api_json(&params).await
    }

    pub async fn query_api_json(&self, params: &HashMap<String, String>) -> anyhow::Result<Value> {
        Ok(self.api.get_query_api_json(params).await?)
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

fn wikimedia_access_token() -> Option<String> {
    std::env::var(WIKIMEDIA_ACCESS_TOKEN_ENV)
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

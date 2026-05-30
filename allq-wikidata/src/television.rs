use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;
use crate::{
    normalize_search_query,
    optional_binding_value,
    required_binding_value,
    sparql_string_escape,
    url_fragment_escape,
    SearchItemsByInstanceOfOptions,
    WikidataClient,
    WikidataItemSearchResult
};

pub const FRANCHISE: &str = "Q196600";
pub const TELEVISION_SERIES_QID: &str = "Q5398426";
pub const TELEVISION_SERIES_SEASON_QID: &str = "Q3464665";

const DEFAULT_EXPANSION_LIMIT: usize = 50;
const DEFAULT_PARENT_CANDIDATE_LIMIT: usize = 10;
const MAX_EXPANSION_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikidataLinkedItem {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikidataTelevisionSeasonSearchResult {
    pub id: String,
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_count: Option<u64>,

    pub series: WikidataLinkedItem,
}

impl WikidataTelevisionSeasonSearchResult {
    pub fn as_item_search_result(&self) -> WikidataItemSearchResult {
        WikidataItemSearchResult {
            id: self.id.clone(),
            label: self.label.clone(),
            description: self.description.clone(),
        }
    }
}

pub async fn search_television_series_seasons_by_series_query_with_options(
    query: &str,
    options: SearchItemsByInstanceOfOptions,
) -> anyhow::Result<Vec<WikidataTelevisionSeasonSearchResult>> {
    anyhow::ensure!(
        !(options.cache_only && options.force_fetch),
        "cache-only and force-fetch cannot be used together"
    );

    let query = normalize_search_query(query)?;
    let output_limit = normalize_limit(options.output_limit);
    let candidate_limit = normalize_candidate_limit(options.candidate_limit);
    let cache_key = television_series_seasons_cache_key(query, output_limit, candidate_limit)?;

    debug!(
        ?query,
        ?output_limit,
        ?candidate_limit,
        ?cache_key,
        ?options
    );

    if !options.force_fetch {
        let cache_client = WikidataClient::new_local_only().await?;

        if let Some(cache) = cache_client.cache_as_ref() {
            if let Some(entry) = cache.get(&cache_key).await? {
                if options.debug_query {
                    eprintln!("debug: tv-season expansion cache hit");
                    eprintln!("debug: cache_key={cache_key}");
                }

                let rows = serde_json::from_str::<Vec<WikidataTelevisionSeasonSearchResult>>(
                    entry.value(),
                )?;
                return Ok(rows);
            }
        }

        if options.debug_query {
            eprintln!("debug: tv-season expansion cache miss");
            eprintln!("debug: cache_key={cache_key}");
        }

        if options.cache_only {
            anyhow::bail!("tv-season expansion result was not found in the local cache");
        }
    } else if options.debug_query {
        eprintln!("debug: tv-season expansion force-fetch enabled; skipping cache read");
        eprintln!("debug: cache_key={cache_key}");
    }

    let sparql = television_series_seasons_by_series_query(
        query,
        output_limit,
        candidate_limit,
    );

    if options.debug_query {
        eprintln!("debug: expansion_route=tv-series-to-seasons");
        eprintln!("debug: parent_type_qid={TELEVISION_SERIES_QID}");
        eprintln!("debug: child_type_qid={TELEVISION_SERIES_SEASON_QID}");
        eprintln!("debug: query={query}");
        eprintln!("debug: output_limit={output_limit}");
        eprintln!("debug: candidate_limit={candidate_limit}");
        eprintln!("debug: SPARQL query:\n{sparql}");
        eprintln!(
            "debug: WDQS URL: https://query.wikidata.org/#{}",
            url_fragment_escape(&sparql),
        );
    }

    let client = WikidataClient::new().await?;
    let response = client.sparql_query_json(&sparql).await?;
    let rows = parse_television_series_season_results(&response)?;

    if options.debug_query {
        eprintln!("debug: tv-season expansion result rows={}", rows.len());
    }

    if let Some(cache) = client.cache_as_ref() {
        let json = serde_json::to_string(&rows)?;
        cache.insert(cache_key, json);

        if options.debug_query {
            eprintln!("debug: saved tv-season expansion result to cache");
        }
    }

    Ok(rows)
}

fn television_series_seasons_by_series_query(
    query: &str,
    output_limit: usize,
    candidate_limit: usize,
) -> String {
    let escaped_query = sparql_string_escape(query);

    format!(
        r#"
SELECT DISTINCT
  ?series
  ?seriesLabel
  ?season
  ?seasonLabel
  ?seasonDescription
  ?ordinal
  ?episodeCount
WHERE {{
  SERVICE wikibase:mwapi {{
    bd:serviceParam wikibase:endpoint "www.wikidata.org" .
    bd:serviceParam wikibase:api "EntitySearch" .
    bd:serviceParam mwapi:search "{escaped_query}" .
    bd:serviceParam mwapi:language "en" .
    bd:serviceParam mwapi:limit "{candidate_limit}" .
    ?series wikibase:apiOutputItem mwapi:item .
  }}

  FILTER EXISTS {{
    ?series wdt:P31/wdt:P279* wd:{TELEVISION_SERIES_QID} .
  }}

  ?series p:P527 ?seasonStatement .
  ?seasonStatement ps:P527 ?season .

  FILTER EXISTS {{
    ?season wdt:P31/wdt:P279* wd:{TELEVISION_SERIES_SEASON_QID} .
  }}

  OPTIONAL {{
    ?seasonStatement pq:P1545 ?ordinal .
  }}

  OPTIONAL {{
    ?season wdt:P1113 ?episodeCount .
  }}

  BIND(
    IF(
      BOUND(?ordinal) && REGEX(STR(?ordinal), "^[0-9]+$"),
      xsd:integer(?ordinal),
      999999
    ) AS ?ordinalSort
  )

  SERVICE wikibase:label {{
    bd:serviceParam wikibase:language "[AUTO_LANGUAGE],en" .
  }}
}}
ORDER BY ?seriesLabel ?ordinalSort ?seasonLabel
LIMIT {output_limit}
"#
    )
}

fn parse_television_series_season_results(
    value: &Value,
) -> anyhow::Result<Vec<WikidataTelevisionSeasonSearchResult>> {
    let bindings = value
        .pointer("/results/bindings")
        .and_then(Value::as_array)
        .context("SPARQL response is missing results.bindings")?;

    bindings
        .iter()
        .map(parse_television_series_season_result)
        .collect()
}

fn parse_television_series_season_result(
    binding: &Value,
) -> anyhow::Result<WikidataTelevisionSeasonSearchResult> {
    let series_uri = required_binding_value(binding, "series")?;
    let season_uri = required_binding_value(binding, "season")?;

    let series_id = item_uri_to_qid(series_uri)?;
    let season_id = item_uri_to_qid(season_uri)?;

    let series_label = optional_binding_value(binding, "seriesLabel")
        .filter(|label| !label.starts_with('Q'))
        .unwrap_or(&series_id)
        .to_string();

    let season_label = optional_binding_value(binding, "seasonLabel")
        .filter(|label| !label.starts_with('Q'))
        .unwrap_or(&season_id)
        .to_string();

    let description = optional_binding_value(binding, "seasonDescription")
        .map(ToString::to_string);

    let ordinal = optional_binding_value(binding, "ordinal")
        .map(ToString::to_string);

    let episode_count = optional_binding_value(binding, "episodeCount")
        .and_then(|value| value.parse::<u64>().ok());

    Ok(WikidataTelevisionSeasonSearchResult {
        id: season_id,
        label: season_label,
        description,
        ordinal,
        episode_count,
        series: WikidataLinkedItem {
            id: series_id,
            label: series_label,
        },
    })
}

fn item_uri_to_qid(uri: &str) -> anyhow::Result<String> {
    let id = uri
        .rsplit('/')
        .next()
        .context("item URI has no path segment")?;

    normalize_item_qid(id)
}

fn normalize_item_qid(qid: &str) -> anyhow::Result<String> {
    let qid = qid.trim();

    anyhow::ensure!(!qid.is_empty(), "Wikidata item QID cannot be empty");
    anyhow::ensure!(
        qid.starts_with('Q') && qid[1..].chars().all(|ch| ch.is_ascii_digit()),
        "expected Wikidata item QID like Q3464665, got: {qid}"
    );

    Ok(qid.to_string())
}

fn normalize_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_EXPANSION_LIMIT)
        .clamp(1, MAX_EXPANSION_LIMIT)
}

fn normalize_candidate_limit(candidate_limit: Option<usize>) -> usize {
    candidate_limit
        .unwrap_or(DEFAULT_PARENT_CANDIDATE_LIMIT)
        .clamp(1, MAX_EXPANSION_LIMIT)
}

fn television_series_seasons_cache_key(
    query: &str,
    output_limit: usize,
    candidate_limit: usize,
) -> anyhow::Result<String> {
    let key_data = serde_json::json!({
        "route": "tv-series-to-seasons",
        "query": query,
        "outputLimit": output_limit,
        "candidateLimit": candidate_limit,
    });

    Ok(format!(
        "wikidata_expansion:v1:{}",
        serde_json::to_string(&key_data)?
    ))
}

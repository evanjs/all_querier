use anyhow::Context;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value;

use crate::WikidataClient;

const DEFAULT_SEARCH_LIMIT: usize = 1;
const DEFAULT_CANDIDATE_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 50;
macro_rules! curated_wikidata_item_types {
    (
        $(
            {
                key: $key:literal,
                qid: $qid:literal,
                label: $label:literal,
                description: $description:literal $(,)?
            }
        ),+ $(,)?
    ) => {
        pub const CURATED_WIKIDATA_ITEM_TYPES: &[WikidataItemType] = &[
            $(
                WikidataItemType {
                    key: $key,
                    qid: $qid,
                    label: $label,
                    description: $description,
                },
            )+
        ];

        pub const CURATED_WIKIDATA_ITEM_TYPE_LABELS: &[&str] = &[
            $(
                $label,
            )+
        ];

        pub const CURATED_WIKIDATA_ITEM_TYPE_KEYS: &[&str] = &[
            $(
                $key,
            )+
        ];
    };
}

curated_wikidata_item_types! {
    {
        key: "anime-tv-series",
        qid: "Q63952888",
        label: "anime television series",
        description: "Japanese anime television series",
    },
    {
        key: "tv-series",
        qid: "Q5398426",
        label: "television series",
        description: "series of connected television program episodes",
    },
    {
        key: "film",
        qid: "Q11424",
        label: "film",
        description: "sequence of images that give the impression of movement",
    },
    {
        key: "video-game",
        qid: "Q7889",
        label: "video game",
        description: "electronic game that involves interaction with a user interface",
    },
    {
        key: "manga-series",
        qid: "Q21198342",
        label: "manga series",
        description: "series of manga volumes or chapters",
    },
    {
        key: "book",
        qid: "Q571",
        label: "book",
        description: "medium for recording information",
    },
    {
        key: "novel",
        qid: "Q8261",
        label: "novel",
        description: "long written narrative fiction",
    },
    {
        key: "album",
        qid: "Q482994",
        label: "album",
        description: "collection of audio recordings issued as a single item",
    },
    {
        key: "song",
        qid: "Q7366",
        label: "song",
        description: "musical composition with vocals",
    },
    {
        key: "character",
        qid: "Q95074",
        label: "character",
        description: "fictional human or non-human character in a narrative work of art",
    },
    {
        key: "taxon",
        qid: "Q16521",
        label: "taxonomic unit",
        description: "group of one or more organism(s), which a taxonomist adjudges to be a unit"
    },
    {
        key: "television-series-episode",
        qid: "Q21191270",
        label: "television series episode",
        description: "single installment of a television series"
    },
    {
        key: "dog-breed",
        qid: "Q39367",
        label: "dog breed",
        description: "group of closely related and visibly similar domestic dogs"
    },
    {
        key: "cat-breed",
        qid: "Q43577",
        label: "cat breed",
        description: "type of pet breed"
    },
    {
        key: "breed",
        qid: "Q38829",
        label: "breed",
        description: "group of domestic animals with a distinctive phenotype"
    },
    {
        key: "video-game-developer",
        qid: "Q210167",
        label: "video game developer",
        description: "software development organization specializing in the creation of video games (for person use Q58287519)"
    },
    {
        key: "video-game-publisher",
        qid: "Q1137109",
        label: "video game publisher",
        description: "company that publishes video games"
    },
    {
        key: "video-game-console",
        qid: "Q8076",
        label: "video game console",
        description: "dedicated electronic device designed primarily for playing video games, typically connected to a display"
    },
    {
        key: "human",
        qid: "Q5",
        label: "human",
        description: "any single member of Homo sapiens, unique extant species of the genus Homo"
    },
    {
        key: "musical-group",
        qid: "Q215380",
        label: "musical group",
        description: "musical ensemble which performs music"
    },
    {
        key: "programming-language",
        qid: "Q9143",
        label: "programming language",
        description: "language for communicating instructions to a machine"
    },
    {
        key: "large-language-model",
        qid: "Q115305900",
        label: "large language model",
        description: "language model built with very large amounts of texts"
    },
    {
        key: "language",
        qid: "Q34770",
        label: "language",
        description: "particular system of communication, often named for the region or peoples that use it",
    },
    {
        key: "television-series-season",
        qid: "Q3464665",
        label: "television series season",
        description: "set of episodes produced for a television series"
    },
    {
        key: "television-series-episode",
        qid: "Q21191270",
        label: "television series episode",
        description: "single installment of a television series"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikidataItemType {
    pub key: &'static str,
    pub qid: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikidataItemSearchResult {
    pub id: String,
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchItemsByInstanceOfOptions {
    pub output_limit: Option<usize>,
    pub candidate_limit: Option<usize>,
    pub include_subclasses: bool,
    pub debug_query: bool,
    pub cache_only: bool,
    pub force_fetch: bool,
}

impl Default for SearchItemsByInstanceOfOptions {
    fn default() -> Self {
        Self {
            output_limit: None,
            candidate_limit: None,
            include_subclasses: true,
            debug_query: false,
            cache_only: false,
            force_fetch: false,
        }
    }
}

pub fn curated_wikidata_item_types() -> &'static [WikidataItemType] {
    CURATED_WIKIDATA_ITEM_TYPES
}

pub fn wikidata_item_type_by_key(key: &str) -> Option<&'static WikidataItemType> {
    let key = key.trim();

    CURATED_WIKIDATA_ITEM_TYPES
        .iter()
        .find(|item_type| item_type.key.eq_ignore_ascii_case(key))
}

pub fn wikidata_item_type_by_key_or_label(value: &str) -> Option<&'static WikidataItemType> {
    let value = value.trim();

    CURATED_WIKIDATA_ITEM_TYPES
        .iter()
        .find(|item_type| {
            item_type.key.eq_ignore_ascii_case(value)
                || item_type.label.eq_ignore_ascii_case(value)
        })
}

pub fn resolve_wikidata_item_type_qid(value: &str) -> anyhow::Result<String> {
    let value = value.trim();

    anyhow::ensure!(!value.is_empty(), "Wikidata item type cannot be empty");

    if value.starts_with('Q') {
        return normalize_item_qid(value);
    }

    if let Some(item_type) = wikidata_item_type_by_key_or_label(value) {
        return Ok(item_type.qid.to_string());
    }

    let known_types = CURATED_WIKIDATA_ITEM_TYPES
        .iter()
        .map(|item_type| {
            if item_type.key.eq_ignore_ascii_case(item_type.label) {
                item_type.key.to_string()
            } else {
                format!("{} ({})", item_type.key, item_type.label)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    anyhow::bail!("unknown Wikidata item type `{value}`; known types: {known_types}")
}

pub async fn search_items_by_curated_type(
    item_type: &str,
    query: &str,
    limit: Option<usize>,
) -> anyhow::Result<Vec<WikidataItemSearchResult>> {
    let type_qid = resolve_wikidata_item_type_qid(item_type)?;

    search_items_by_instance_of(&type_qid, query, limit, true).await
}

pub async fn search_items_by_instance_of(
    type_qid: &str,
    query: &str,
    limit: Option<usize>,
    include_subclasses: bool,
) -> anyhow::Result<Vec<WikidataItemSearchResult>> {
    search_items_by_instance_of_with_options(
        type_qid,
        query,
        SearchItemsByInstanceOfOptions {
            output_limit: limit,
            candidate_limit: None,
            include_subclasses,
            debug_query: false,
            cache_only: false,
            force_fetch: false,
        },
    )
        .await
}

pub async fn search_items_by_instance_of_with_options(
    type_qid: &str,
    query: &str,
    options: SearchItemsByInstanceOfOptions,
) -> anyhow::Result<Vec<WikidataItemSearchResult>> {
    anyhow::ensure!(
        !(options.cache_only && options.force_fetch),
        "cache-only and force-fetch cannot be used together"
    );

    let type_qid = normalize_item_qid(type_qid)?;
    let query = normalize_search_query(query)?;
    let output_limit = normalize_limit(options.output_limit);
    let candidate_limit = normalize_candidate_limit(options.candidate_limit, output_limit);
    let cache_key = search_items_by_instance_of_cache_key(
        &type_qid,
        query,
        output_limit,
        candidate_limit,
        options.include_subclasses,
    )?;

    if !options.force_fetch {
        let cache_client = WikidataClient::new_local_only().await?;

        if let Some(cache) = cache_client.cache_as_ref() {
            if let Some(entry) = cache.get(&cache_key).await? {
                if options.debug_query {
                    eprintln!("debug: search-item cache hit");
                    eprintln!("debug: cache_key={cache_key}");
                }

                let s = entry.value().clone();
                let rows = serde_json::from_str::<Vec<WikidataItemSearchResult>>(&s)?;
                return Ok(rows);
            }
        }

        if options.debug_query {
            eprintln!("debug: search-item cache miss");
            eprintln!("debug: cache_key={cache_key}");
        }

        if options.cache_only {
            anyhow::bail!("search-item result was not found in the local cache");
        }
    } else if options.debug_query {
        eprintln!("debug: search-item force-fetch enabled; skipping cache read");
        eprintln!("debug: cache_key={cache_key}");
    }

    let sparql = search_items_by_instance_of_query(
        &type_qid,
        query,
        output_limit,
        candidate_limit,
        options.include_subclasses,
    );

    if options.debug_query {
        eprintln!("debug: type_qid={type_qid}");
        eprintln!("debug: query={query}");
        eprintln!("debug: output_limit={output_limit}");
        eprintln!("debug: candidate_limit={candidate_limit}");
        eprintln!("debug: include_subclasses={}", options.include_subclasses);
        eprintln!("debug: SPARQL query:\n{sparql}");
        eprintln!(
            "debug: WDQS URL: https://query.wikidata.org/#{}",
            url_fragment_escape(&sparql),
        );
    }

    let client = WikidataClient::new().await?;
    let res = client.sparql_query_json(&sparql).await?;

    if options.debug_query {
        let binding_count = res
            .pointer("/results/bindings")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);

        eprintln!("debug: SPARQL result bindings={binding_count}");
    }

    let rows = parse_item_search_results(&res)?;

    if let Some(cache) = client.cache_as_ref() {
        let s = serde_json::to_string(&rows)?;
        cache.insert(cache_key, s);

        if options.debug_query {
            eprintln!("debug: saved search-item result to cache");
        }
    }

    Ok(rows)
}

fn search_items_by_instance_of_query(
    type_qid: &str,
    query: &str,
    output_limit: usize,
    candidate_limit: usize,
    include_subclasses: bool,
) -> String {
    let escaped_query = sparql_string_escape(query);
    let instance_filter = if include_subclasses {
        format!("?item wdt:P31/wdt:P279* wd:{type_qid} .")
    } else {
        format!("?item wdt:P31 wd:{type_qid} .")
    };

    format!(
        r#"
SELECT DISTINCT ?item ?itemLabel ?itemDescription WHERE {{
  SERVICE wikibase:mwapi {{
    bd:serviceParam wikibase:endpoint "www.wikidata.org" .
    bd:serviceParam wikibase:api "EntitySearch" .
    bd:serviceParam mwapi:search "{escaped_query}" .
    bd:serviceParam mwapi:language "en" .
    bd:serviceParam mwapi:limit "{candidate_limit}" .
    ?item wikibase:apiOutputItem mwapi:item .
  }}

  {instance_filter}

  SERVICE wikibase:label {{
    bd:serviceParam wikibase:language "[AUTO_LANGUAGE],en" .
  }}
}}
LIMIT {output_limit}
"#
    )
}

fn parse_item_search_results(value: &Value) -> anyhow::Result<Vec<WikidataItemSearchResult>> {
    let bindings = value
        .pointer("/results/bindings")
        .and_then(Value::as_array)
        .context("SPARQL response is missing results.bindings")?;

    bindings
        .iter()
        .map(parse_item_search_result)
        .collect()
}

fn parse_item_search_result(binding: &Value) -> anyhow::Result<WikidataItemSearchResult> {
    let item_uri = required_binding_value(binding, "item")?;
    let id = item_uri_to_qid(item_uri)?;

    let label = optional_binding_value(binding, "itemLabel")
        .filter(|label| !label.starts_with('Q'))
        .unwrap_or(&id)
        .to_string();

    let description = optional_binding_value(binding, "itemDescription")
        .map(ToString::to_string);

    Ok(WikidataItemSearchResult {
        id,
        label,
        description,
    })
}

fn required_binding_value<'a>(binding: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .with_context(|| format!("SPARQL row is missing binding value: {key}"))
}

fn optional_binding_value<'a>(binding: &'a Value, key: &str) -> Option<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
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
        "expected Wikidata item QID like Q63952888, got: {qid}"
    );

    Ok(qid.to_string())
}

fn normalize_search_query(query: &str) -> anyhow::Result<&str> {
    let query = query.trim();

    anyhow::ensure!(!query.is_empty(), "search query cannot be empty");

    Ok(query)
}

fn normalize_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT)
}

fn normalize_candidate_limit(candidate_limit: Option<usize>, output_limit: usize) -> usize {
    candidate_limit
        .unwrap_or(DEFAULT_CANDIDATE_LIMIT)
        .max(output_limit)
        .clamp(1, MAX_SEARCH_LIMIT)
}

fn search_items_by_instance_of_cache_key(
    type_qid: &str,
    query: &str,
    output_limit: usize,
    candidate_limit: usize,
    include_subclasses: bool,
) -> anyhow::Result<String> {
    let key_data = serde_json::json!({
        "typeQid": type_qid,
        "query": query,
        "outputLimit": output_limit,
        "candidateLimit": candidate_limit,
        "includeSubclasses": include_subclasses,
    });

    Ok(format!(
        "search_items_by_instance_of:v1:{}",
        serde_json::to_string(&key_data)?
    ))
}

fn url_fragment_escape(value: &str) -> String {
    let mut escaped = String::new();

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~' => escaped.push(byte as char),
            b' ' => escaped.push_str("%20"),
            b'\n' => escaped.push_str("%0A"),
            b'\r' => escaped.push_str("%0D"),
            b'\t' => escaped.push_str("%09"),
            _ => escaped.push_str(&format!("%{byte:02X}")),
        }
    }

    escaped
}

fn sparql_string_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}
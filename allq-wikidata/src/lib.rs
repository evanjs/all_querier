mod cache;
mod client;
mod smoke;
mod listproperties;
mod properties;
mod itemtypes;
mod externallinks;
mod search_provider;
mod television;

use anyhow::Context;
pub use cache::{
    create_wikidata_cache, WikidataCache,
};
pub use client::{
    wikidata_api, WikidataClient, WikidataEntityLookupMode, ENTITY_QUERY_PROPS, WIKIDATA_API_URL,
};
pub use externallinks::{
    add_external_links_to_entities,
    add_external_links_to_entity,
    add_external_links_to_wbgetentities_response,
    external_ids_by_qid,
    external_ids_for_entity,
    ExternalId,
};
pub use itemtypes::{
    curated_wikidata_item_types, resolve_wikidata_item_type_qid,
    search_items_by_curated_type, search_items_by_instance_of,
    search_items_by_instance_of_with_options, wikidata_item_type_by_key, wikidata_item_type_by_key_or_label,
    SearchItemsByInstanceOfOptions, WikidataItemSearchResult,
    WikidataItemType, CURATED_WIKIDATA_ITEM_TYPES,
    CURATED_WIKIDATA_ITEM_TYPE_KEYS, CURATED_WIKIDATA_ITEM_TYPE_LABELS,
};
pub use listproperties::{
    fetch_listproperties_rows_json,
    list_properties_id_name_description_json,
    IdNameDescription,
};
pub use properties::{
    DatatypeKey, Properties, Property,
};
pub use search_provider::WikidataSearchProvider;
use serde_json::Value;
pub use smoke::{
    retrieve_entity_by_qid, retrieve_entity_by_qid_with_options, smoke_test,
};
pub use television::{
    search_television_series_seasons_by_series_query_with_options,
    WikidataLinkedItem,
    WikidataTelevisionSeasonSearchResult,
    FRANCHISE,
    TELEVISION_SERIES_QID,
    TELEVISION_SERIES_SEASON_QID,
};

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


pub(crate) fn required_binding_value<'a>(binding: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
        .with_context(|| format!("SPARQL row is missing binding value: {key}"))
}

pub(crate) fn optional_binding_value<'a>(binding: &'a Value, key: &str) -> Option<&'a str> {
    binding
        .get(key)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
}

fn normalize_search_query(query: &str) -> anyhow::Result<&str> {
    let query = query.trim();

    anyhow::ensure!(!query.is_empty(), "search query cannot be empty");

    Ok(query)
}
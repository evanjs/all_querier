use std::collections::HashMap;

use anyhow::Context;
use serde_json::{
    Map,
    Value,
};

use crate::{
    WikidataClient,
    WikidataEntityLookupMode,
};

const FORMATTER_URL_PROPERTY: &str = "P1630";
const THIRD_PARTY_FORMATTER_URL_PROPERTY: &str = "P3303";
const URL_MATCH_PATTERN_PROPERTY: &str = "P8966";
const ID_REGEX_PROPERTY: &str = "P1793";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalId {
    pub wikidata_qid: Option<String>,
    pub property_id: String,
    pub property_name: Option<String>,
    pub source: Option<String>,
    pub value: String,
    pub urls: Vec<String>,
    pub formatter_urls: Vec<String>,
    pub third_party_formatter_urls: Vec<String>,
    pub url_match_patterns: Vec<String>,
    pub id_regexes: Vec<String>,
    pub supported: bool,
}

pub async fn external_ids_by_qid(qid: &str, client: &WikidataClient, lookup_mode: WikidataEntityLookupMode) -> anyhow::Result<Vec<ExternalId>> {
    let qid = qid.trim();
    let response = client.entity_by_qid_with_mode(qid, lookup_mode).await?;
    let entity = response.get("entities").and_then(|e| e.get(qid)).context("entity not found")?;
    external_ids_for_entity(entity, client, lookup_mode).await
}

async fn fetch_properties_external_link_metadata(
    client: &WikidataClient,
    property_ids: &[String],
    lookup_mode: WikidataEntityLookupMode,
) -> anyhow::Result<HashMap<String, PropertyExternalLinkMetadata>> {
    let response = client
        .entities_by_qids_with_mode(property_ids, lookup_mode)
        .await?;

    let entities = response
        .get("entities")
        .and_then(Value::as_object)
        .context("property entities response did not include entities")?;

    let mut metadata_by_property = HashMap::new();

    for property_id in property_ids {
        let property_entity = entities
            .get(property_id)
            .with_context(|| {
                format!("Wikidata property response did not include {property_id}")
            })?;

        metadata_by_property.insert(
            property_id.clone(),
            PropertyExternalLinkMetadata {
                label: english_label(property_entity),
                formatter_urls: string_claim_values(property_entity, FORMATTER_URL_PROPERTY),
                third_party_formatter_urls: string_claim_values(
                    property_entity,
                    THIRD_PARTY_FORMATTER_URL_PROPERTY,
                ),
                url_match_patterns: string_claim_values(property_entity, URL_MATCH_PATTERN_PROPERTY),
                id_regexes: string_claim_values(property_entity, ID_REGEX_PROPERTY),
            },
        );
    }

    Ok(metadata_by_property)
}

pub async fn external_ids_for_entity(entity: &Value, client: &WikidataClient, lookup_mode: WikidataEntityLookupMode) -> anyhow::Result<Vec<ExternalId>> {
    let claims = collect_external_id_claims(entity);
    let wikidata_qid = entity.get("id").and_then(Value::as_str).map(ToString::to_string);
    let mut metadata_by_property = HashMap::new();
    let mut external_ids = Vec::new();

    for claim in claims {
        let metadata = metadata_by_property.entry(claim.property_id.clone()).or_insert(
            fetch_property_external_link_metadata(client, &claim.property_id, lookup_mode).await?
        );

        let mut urls = Vec::new();
        append_formatted_urls(&mut urls, &metadata.formatter_urls, &claim.value);
        append_formatted_urls(&mut urls, &metadata.third_party_formatter_urls, &claim.value);

        let source = match claim.property_id.as_str() {
            "P13031" => Some("mywaifulist".to_string()),
            "P1733" => Some("steam".to_string()),
            _ => None,
        };

        external_ids.push(ExternalId {
            wikidata_qid: wikidata_qid.clone(),
            property_id: claim.property_id,
            property_name: claim.property_name.or_else(|| metadata.label.clone()),
            supported: source.is_some(),
            source,
            value: claim.value,
            urls,
            formatter_urls: metadata.formatter_urls.clone(),
            third_party_formatter_urls: metadata.third_party_formatter_urls.clone(),
            url_match_patterns: metadata.url_match_patterns.clone(),
            id_regexes: metadata.id_regexes.clone(),
        });
    }
    Ok(external_ids)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExternalIdClaim {
    property_id: String,
    property_name: Option<String>,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PropertyExternalLinkMetadata {
    label: Option<String>,
    formatter_urls: Vec<String>,
    third_party_formatter_urls: Vec<String>,
    url_match_patterns: Vec<String>,
    id_regexes: Vec<String>,
}

pub async fn add_external_links_to_wbgetentities_response(
    response: &mut Value,
    client: &WikidataClient,
    lookup_mode: WikidataEntityLookupMode,
) -> anyhow::Result<()> {
    let Some(entities) = response
        .get_mut("entities")
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };

    for entity in entities.values_mut() {
        add_external_links_to_entity(entity, client, lookup_mode).await?;
    }

    Ok(())
}

pub async fn add_external_links_to_entities(
    entities: &mut [Value],
    client: &WikidataClient,
    lookup_mode: WikidataEntityLookupMode,
) -> anyhow::Result<()> {
    for entity in entities {
        add_external_links_to_entity(entity, client, lookup_mode).await?;
    }

    Ok(())
}

pub async fn add_external_links_to_entity(
    entity: &mut Value,
    client: &WikidataClient,
    lookup_mode: WikidataEntityLookupMode,
) -> anyhow::Result<()> {
    let claims = collect_external_id_claims(entity);

    if claims.is_empty() {
        return Ok(());
    }

    let mut property_ids = claims
        .iter()
        .map(|claim| claim.property_id.clone())
        .collect::<Vec<_>>();

    property_ids.sort();
    property_ids.dedup();

    let metadata_by_property = fetch_properties_external_link_metadata(
        client,
        &property_ids,
        lookup_mode,
    )
        .await?;

    let mut external_links = Vec::new();

    for claim in claims {
        let metadata = metadata_by_property
            .get(&claim.property_id)
            .with_context(|| {
                format!(
                    "external-link metadata response did not include {}",
                    claim.property_id,
                )
            })?;

        let mut urls = Vec::new();
        append_formatted_urls(&mut urls, &metadata.formatter_urls, &claim.value);
        append_formatted_urls(&mut urls, &metadata.third_party_formatter_urls, &claim.value);

        if urls.is_empty()
            && metadata.formatter_urls.is_empty()
            && metadata.third_party_formatter_urls.is_empty()
            && metadata.url_match_patterns.is_empty()
            && metadata.id_regexes.is_empty()
        {
            continue;
        }

        let mut link = Map::new();
        link.insert("property".to_string(), Value::String(claim.property_id));
        link.insert("value".to_string(), Value::String(claim.value));

        if let Some(property_name) = claim.property_name.or_else(|| metadata.label.clone()) {
            link.insert("propertyName".to_string(), Value::String(property_name));
        }

        insert_string_array_if_not_empty(&mut link, "urls", &urls);
        insert_string_array_if_not_empty(&mut link, "formatterUrls", &metadata.formatter_urls);
        insert_string_array_if_not_empty(
            &mut link,
            "thirdPartyFormatterUrls",
            &metadata.third_party_formatter_urls,
        );
        insert_string_array_if_not_empty(
            &mut link,
            "urlMatchPatterns",
            &metadata.url_match_patterns,
        );
        insert_string_array_if_not_empty(&mut link, "idRegexes", &metadata.id_regexes);

        external_links.push(Value::Object(link));
    }

    if external_links.is_empty() {
        return Ok(());
    }

    let entity = entity
        .as_object_mut()
        .context("Wikidata entity is not a JSON object")?;

    entity.insert("externalLinks".to_string(), Value::Array(external_links));

    Ok(())
}

fn collect_external_id_claims(entity: &Value) -> Vec<ExternalIdClaim> {
    let Some(claims) = entity
        .get("claims")
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };

    let property_names = entity
        .get("propertyNames")
        .and_then(Value::as_object);

    let mut external_id_claims = Vec::new();

    for (property_id, statements) in claims {
        let Some(statements) = statements.as_array() else {
            continue;
        };

        for statement in statements {
            let Some(mainsnak) = statement.get("mainsnak") else {
                continue;
            };

            let is_external_id = mainsnak
                .get("datatype")
                .and_then(Value::as_str)
                == Some("external-id");

            if !is_external_id {
                continue;
            }

            let Some(value) = mainsnak
                .pointer("/datavalue/value")
                .and_then(Value::as_str)
            else {
                continue;
            };

            let property_name = mainsnak
                .get("propertyName")
                .and_then(Value::as_str)
                .or_else(|| {
                    property_names
                        .and_then(|property_names| property_names.get(property_id))
                        .and_then(Value::as_str)
                })
                .map(ToString::to_string);

            external_id_claims.push(ExternalIdClaim {
                property_id: property_id.clone(),
                property_name,
                value: value.to_string(),
            });
        }
    }

    external_id_claims
}

async fn fetch_property_external_link_metadata(
    client: &WikidataClient,
    property_id: &str,
    lookup_mode: WikidataEntityLookupMode,
) -> anyhow::Result<PropertyExternalLinkMetadata> {
    let response = client
        .entity_by_qid_with_mode(property_id, lookup_mode)
        .await?;

    let property_entity = response
        .get("entities")
        .and_then(|entities| entities.get(property_id))
        .with_context(|| {
            format!("Wikidata property response did not include {property_id}")
        })?;

    Ok(PropertyExternalLinkMetadata {
        label: english_label(property_entity),
        formatter_urls: string_claim_values(property_entity, FORMATTER_URL_PROPERTY),
        third_party_formatter_urls: string_claim_values(
            property_entity,
            THIRD_PARTY_FORMATTER_URL_PROPERTY,
        ),
        url_match_patterns: string_claim_values(property_entity, URL_MATCH_PATTERN_PROPERTY),
        id_regexes: string_claim_values(property_entity, ID_REGEX_PROPERTY),
    })
}

fn english_label(entity: &Value) -> Option<String> {
    entity
        .pointer("/labels/en/value")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn string_claim_values(entity: &Value, property_id: &str) -> Vec<String> {
    let Some(statements) = entity
        .get("claims")
        .and_then(|claims| claims.get(property_id))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut values = Vec::new();

    for statement in statements {
        let Some(value) = statement
            .pointer("/mainsnak/datavalue/value")
            .and_then(Value::as_str)
        else {
            continue;
        };

        push_unique(&mut values, value.to_string());
    }

    values
}

fn append_formatted_urls(urls: &mut Vec<String>, formatter_urls: &[String], value: &str) {
    for formatter_url in formatter_urls {
        push_unique(urls, formatter_url.replace("$1", value));
    }
}

fn insert_string_array_if_not_empty(object: &mut Map<String, Value>, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }

    object.insert(
        key.to_string(),
        Value::Array(
            values
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}
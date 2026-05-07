use std::collections::HashMap;
use std::time::Instant;
use allq_providers::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderPageData,
    resolve_provider_link,
};
use serde_json::{
    Map,
    Value,
};
use tracing::debug;

pub struct WikidataQueryOptions<'a> {
    pub item_type: Option<&'a str>,
    pub type_qid: Option<&'a str>,
    pub query: &'a str,
    pub link: Option<&'a str>,
    pub limit: usize,
    pub candidate_limit: Option<usize>,
    pub cache_only: bool,
    pub force_fetch: bool,
    pub direct_only: bool,
    pub debug_query: bool,
    pub annotate_properties: bool,
    pub enrich_external_links: bool,
}

pub enum WikidataQueryResult {
    Entities {
        rows: Vec<allq_wikidata::WikidataItemSearchResult>,
        entities: Vec<Value>,
    },
    FollowedLink {
        value: Value,
    },
}

impl WikidataQueryResult {
    pub fn into_json_value(self) -> Value {
        match self {
            Self::Entities { entities, .. } => Value::Array(entities),
            Self::FollowedLink { value } => value,
        }
    }

    pub fn to_json_value(&self) -> Value {
        match self {
            Self::Entities { entities, .. } => Value::Array(entities.clone()),
            Self::FollowedLink { value } => value.clone(),
        }
    }
}

pub async fn query_wikidata(
    options: WikidataQueryOptions<'_>,
) -> anyhow::Result<WikidataQueryResult> {
    let type_qid = resolve_query_type_qid(options.item_type, options.type_qid)?;

    let search_started_at = Instant::now();

    let rows = allq_wikidata::search_items_by_instance_of_with_options(
        &type_qid,
        options.query,
        allq_wikidata::SearchItemsByInstanceOfOptions {
            output_limit: Some(options.limit),
            candidate_limit: options.candidate_limit,
            include_subclasses: !options.direct_only,
            debug_query: options.debug_query,
            cache_only: options.cache_only,
            force_fetch: options.force_fetch,
        },
    )
        .await?;

    if options.debug_query {
        eprintln!("debug: search-item elapsed={:?}", search_started_at.elapsed());
    }

    let lookup_mode = wikidata_lookup_mode(options.cache_only, options.force_fetch);

    if options.debug_query {
        eprintln!(
            "debug: hydrating {} search-item result(s) via batched entity-by-qid",
            rows.len()
        );
        eprintln!("debug: entity lookup mode={lookup_mode:?}");
    }

    let client = wikidata_client(options.cache_only).await?;

    let hydration_started_at = Instant::now();
    let mut entities = hydrate_search_item_entities(&client, &rows, lookup_mode).await?;

    if options.debug_query {
        eprintln!("debug: hydration elapsed={:?}", hydration_started_at.elapsed());
    }

    if let Some(link) = options.link {
        let provider_http_client = ProviderHttpClient::new()?;
        let value = follow_search_item_link(
            &client,
            &provider_http_client,
            &entities,
            lookup_mode,
            link,
        )
            .await?;

        return Ok(WikidataQueryResult::FollowedLink { value });
    }

    if options.enrich_external_links {
        let external_links_started_at = Instant::now();

        if options.debug_query {
            eprintln!(
                "debug: enriching external links for {} hydrated entity/entities",
                entities.len()
            );
        }

        allq_wikidata::add_external_links_to_entities(
            &mut entities,
            &client,
            lookup_mode,
        )
            .await?;

        if options.debug_query {
            eprintln!(
                "debug: external-link enrichment elapsed={:?}",
                external_links_started_at.elapsed()
            );
        }
    }

    if options.annotate_properties {
        let property_names = wikidata_property_names_by_id().await?;
        annotate_entities_with_property_names(&mut entities, &property_names);
    }

    Ok(WikidataQueryResult::Entities { rows, entities })
}

pub fn resolve_query_type_qid(
    item_type: Option<&str>,
    type_qid: Option<&str>,
) -> anyhow::Result<String> {
    match (item_type, type_qid) {
        (Some(item_type), None) => allq_wikidata::resolve_wikidata_item_type_qid(item_type),
        (None, Some(type_qid)) => allq_wikidata::resolve_wikidata_item_type_qid(type_qid),
        (None, None) => anyhow::bail!("provide either --type or --type-qid"),
        (Some(_), Some(_)) => anyhow::bail!("provide only one of --type or --type-qid"),
    }
}

pub fn wikidata_lookup_mode(
    cache_only: bool,
    force_fetch: bool,
) -> allq_wikidata::WikidataEntityLookupMode {
    if cache_only {
        allq_wikidata::WikidataEntityLookupMode::CacheOnly
    } else if force_fetch {
        allq_wikidata::WikidataEntityLookupMode::ForceFetch
    } else {
        allq_wikidata::WikidataEntityLookupMode::NetworkFallback
    }
}

pub async fn wikidata_client(
    cache_only: bool,
) -> anyhow::Result<allq_wikidata::WikidataClient> {
    if cache_only {
        allq_wikidata::WikidataClient::new_local_only().await
    } else {
        allq_wikidata::WikidataClient::new().await
    }
}

fn provider_page_cache_key(source: &str, value: &str) -> String {
    format!("external_page:{source}:{value}")
}

pub async fn hydrate_search_item_entities(
    client: &allq_wikidata::WikidataClient,
    rows: &[allq_wikidata::WikidataItemSearchResult],
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
) -> anyhow::Result<Vec<Value>> {
    let qids = rows
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();

    let response = client
        .entities_by_qids_with_mode(&qids, lookup_mode)
        .await?;

    rows
        .iter()
        .map(|row| {
            response
                .get("entities")
                .and_then(|entities| entities.get(&row.id))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Wikidata entity response did not include {}", row.id))
        })
        .collect()
}

pub async fn follow_search_item_link(
    client: &allq_wikidata::WikidataClient,
    provider_http_client: &ProviderHttpClient,
    entities: &[Value],
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
    link: &str,
) -> anyhow::Result<Value> {
    let route = resolve_provider_link(link)?;

    debug!(
        link,
        source = route.source(),
        property_id = route.property_id(),
        "resolved external link route",
    );

    let entity = entities
        .first()
        .ok_or_else(|| anyhow::anyhow!("search-item produced no entities to follow"))?;

    let external_ids = allq_wikidata::external_ids_for_entity(
        entity,
        client,
        lookup_mode,
    )
        .await?;

    let external_id = external_ids
        .iter()
        .find(|external_id| {
            external_id.property_id == route.property_id()
                && external_id.source.as_deref() == Some(route.source())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "resolved Wikidata item has no supported {link} external ID; expected {} / {}",
                route.property_id(),
                route.source(),
            )
        })?;

    debug!(
        wikidata_qid = external_id.wikidata_qid.as_deref().unwrap_or(""),
        property_id = external_id.property_id,
        source = external_id.source.as_deref().unwrap_or(""),
        value = external_id.value,
        "resolved provider external ID",
    );

    let provider = route.provider();
    let page_data = fetch_provider_page_data_with_cache(
        client,
        provider_http_client,
        provider,
        &external_id.value,
        lookup_mode,
    )
        .await?;

    provider.parse_page_data(&page_data)
}

pub async fn fetch_provider_page_data_with_cache<P>(
    client: &allq_wikidata::WikidataClient,
    provider_http_client: &ProviderHttpClient,
    provider: &P,
    value: &str,
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
) -> anyhow::Result<ProviderPageData>
where
    P: ExternalIdPageProvider + Sync + ?Sized,
{
    let cache_key = provider_page_cache_key(provider.source(), value);

    if lookup_mode != allq_wikidata::WikidataEntityLookupMode::ForceFetch {
        if let Some(cache) = client.cache_as_ref() {
            if let Some(entry) = cache.get(&cache_key).await? {
                debug!(
                        cache_key = %cache_key,
                        source = provider.source(),
                        value,
                        "provider page cache hit",
                    );

                return Ok(ProviderPageData {
                    source: provider.source(),
                    url: provider.page_url(value),
                    body: entry.value().clone(),
                });
            }
        }
    }

    if lookup_mode == allq_wikidata::WikidataEntityLookupMode::CacheOnly {
        anyhow::bail!(
                "provider page {}:{value} was not found in the local cache",
                provider.source(),
            );
    }

    debug!(
            cache_key = %cache_key,
            source = provider.source(),
            value,
            "provider page cache miss; fetching",
        );

    let page_data = provider.fetch_page_data(provider_http_client, value).await?;

    if let Some(cache) = client.cache_as_ref() {
        cache.insert(cache_key.clone(), page_data.body.clone());

        debug!(
                cache_key = %cache_key,
                source = provider.source(),
                value,
                bytes = page_data.body.len(),
                "stored provider page in cache",
            );
    }

    Ok(page_data)
}

pub async fn wikidata_property_names_by_id() -> anyhow::Result<HashMap<String, String>> {
    let properties = allq_wikidata::list_properties_id_name_description_json(false).await?;

    Ok(properties
        .into_iter()
        .map(|property| (property.id, property.name))
        .collect())
}

pub fn annotate_entities_with_property_names(
    entities: &mut [Value],
    property_names: &HashMap<String, String>,
) {
    for entity in entities {
        add_entity_claim_property_names(entity, property_names);
        annotate_value_property_names(entity, property_names);
    }
}

pub fn add_entity_claim_property_names(
    entity: &mut Value,
    property_names: &HashMap<String, String>,
) {
    let Some(claims) = entity
        .get("claims")
        .and_then(Value::as_object)
    else {
        return;
    };

    let names = claims
        .keys()
        .filter_map(|property_id| {
            property_names
                .get(property_id)
                .map(|name| (property_id.clone(), Value::String(name.clone())))
        })
        .collect::<Map<String, Value>>();

    if names.is_empty() {
        return;
    }

    if let Some(entity) = entity.as_object_mut() {
        entity.insert("propertyNames".to_string(), Value::Object(names));
    }
}

pub fn annotate_value_property_names(
    value: &mut Value,
    property_names: &HashMap<String, String>,
) {
    match value {
        Value::Object(object) => {
            if let Some(property_id) = object
                .get("property")
                .and_then(Value::as_str)
            {
                if let Some(property_name) = property_names.get(property_id) {
                    object.insert(
                        "propertyName".to_string(),
                        Value::String(property_name.clone()),
                    );
                }
            }

            for value in object.values_mut() {
                annotate_value_property_names(value, property_names);
            }
        }
        Value::Array(values) => {
            for value in values {
                annotate_value_property_names(value, property_names);
            }
        }
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::String(_) => {}
    }
}
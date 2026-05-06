use allq_providers::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderPageData,
    resolve_provider_link,
};
use nu_plugin::{
    EngineInterface,
    EvaluatedCall,
    SimplePluginCommand,
};
use nu_protocol::{Category, Completion, Example, Flag, LabeledError, Signature, Span, SyntaxShape, Value};
use crate::AllQuerierPlugin;

pub struct QueryWikidata;

impl SimplePluginCommand for QueryWikidata {
    type Plugin = AllQuerierPlugin;

    fn name(&self) -> &str {
        "query wikidata"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "query",
                SyntaxShape::String,
                "Search query/title, e.g. 'tier harribel'",
            )
            .param(
                Flag::new("type")
                    .short('t')
                    .arg(SyntaxShape::String)
                    .desc("Curated Wikidata item type, e.g. character")
                    .completion(
                        Completion::new_list(
                            allq_wikidata::CURATED_WIKIDATA_ITEM_TYPE_LABELS
                        )
                    )
            )
            .param(
                Flag::new("link")
                    .short('L')
                    .arg(SyntaxShape::String)
                    .desc("Supported external provider link alias, e.g. waifu")
                    .completion(
                        Completion::new_list(
                            allq_providers::SUPPORTED_PROVIDER_LINK_PRIMARY_ALIASES
                        )
                    )
            )
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of Wikidata search results to consider",
                Some('n'),
            )
            .switch(
                "cache-only",
                "Only read from the local cache; do not call Wikidata/provider APIs",
                None,
            )
            .switch(
                "force-fetch",
                "Ignore cached data and fetch from Wikidata/provider APIs",
                None,
            )
            .switch(
                "direct-only",
                "Only match direct P31 values; do not include subclasses via P279",
                None,
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Search Wikidata by curated item type and optionally follow a supported external provider link"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: r#"query wikidata --type character -L waifu "tier harribel""#,
            description: "Find a character on Wikidata and return the supported MyWaifuList page data",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &AllQuerierPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let query: String = call.req(0)?;
        let item_type: String = call
            .get_flag("type")?
            .unwrap_or_else(|| "character".to_string());
        let link: Option<String> = call.get_flag("link")?;
        let limit = call
            .get_flag::<i64>("limit")?
            .and_then(|limit| usize::try_from(limit).ok())
            .unwrap_or(1);

        let cache_only = call.has_flag("cache-only")?;
        let force_fetch = call.has_flag("force-fetch")?;
        let direct_only = call.has_flag("direct-only")?;
        let head = call.head;

        if cache_only && force_fetch {
            return Err(LabeledError::new("Conflicting flags")
                .with_label("--cache-only and --force-fetch cannot be used together", head));
        }

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| labeled_error(head, "Failed to create Tokio runtime", error))?;

        let value = runtime
            .block_on(run_query_wikidata(
                &item_type,
                &query,
                link.as_deref(),
                limit,
                cache_only,
                force_fetch,
                direct_only,
            ))
            .map_err(|error| labeled_error(head, "Wikidata query failed", error))?;

        serde_json_to_nu_value(value, head)
            .map_err(|error| labeled_error(head, "Failed to convert JSON to Nushell value", error))
    }
}

async fn run_query_wikidata(
    item_type: &str,
    query: &str,
    link: Option<&str>,
    limit: usize,
    cache_only: bool,
    force_fetch: bool,
    direct_only: bool,
) -> anyhow::Result<serde_json::Value> {
    let type_qid = allq_wikidata::resolve_wikidata_item_type_qid(item_type)?;

    let rows = allq_wikidata::search_items_by_instance_of_with_options(
        &type_qid,
        query,
        allq_wikidata::SearchItemsByInstanceOfOptions {
            output_limit: Some(limit),
            candidate_limit: None,
            include_subclasses: !direct_only,
            debug_query: false,
            cache_only,
            force_fetch,
        },
    )
        .await?;

    let lookup_mode = if cache_only {
        allq_wikidata::WikidataEntityLookupMode::CacheOnly
    } else if force_fetch {
        allq_wikidata::WikidataEntityLookupMode::ForceFetch
    } else {
        allq_wikidata::WikidataEntityLookupMode::NetworkFallback
    };

    let client = if cache_only {
        allq_wikidata::WikidataClient::new_local_only().await?
    } else {
        allq_wikidata::WikidataClient::new().await?
    };

    let entities = hydrate_search_item_entities(&client, &rows, lookup_mode).await?;

    if let Some(link) = link {
        let provider_http_client = ProviderHttpClient::new()?;

        return follow_search_item_link(
            &client,
            &provider_http_client,
            &entities,
            lookup_mode,
            link,
        )
            .await;
    }

    let mut entities = entities;
    allq_wikidata::add_external_links_to_entities(
        &mut entities,
        &client,
        lookup_mode,
    )
        .await?;

    Ok(serde_json::Value::Array(entities))
}

async fn hydrate_search_item_entities(
    client: &allq_wikidata::WikidataClient,
    rows: &[allq_wikidata::WikidataItemSearchResult],
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut entities = Vec::with_capacity(rows.len());

    for row in rows {
        let response = client.entity_by_qid_with_mode(&row.id, lookup_mode).await?;
        let entity = response
            .get("entities")
            .and_then(|entities| entities.get(&row.id))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Wikidata entity response did not include {}", row.id))?;

        entities.push(entity);
    }

    Ok(entities)
}

async fn follow_search_item_link(
    client: &allq_wikidata::WikidataClient,
    provider_http_client: &ProviderHttpClient,
    entities: &[serde_json::Value],
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
    link: &str,
) -> anyhow::Result<serde_json::Value> {
    let route = resolve_provider_link(link)?;

    let entity = entities
        .first()
        .ok_or_else(|| anyhow::anyhow!("Wikidata search produced no entities to follow"))?;

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

async fn fetch_provider_page_data_with_cache<P>(
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

    let page_data = provider.fetch_page_data(provider_http_client, value).await?;

    if let Some(cache) = client.cache_as_ref() {
        cache.insert(cache_key, page_data.body.clone());
    }

    Ok(page_data)
}

fn provider_page_cache_key(source: &str, value: &str) -> String {
    format!("external_page:{source}:{value}")
}

fn serde_json_to_nu_value(
    value: serde_json::Value,
    span: Span,
) -> anyhow::Result<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::nothing(span)),
        serde_json::Value::Bool(value) => Ok(Value::bool(value, span)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Value::int(value, span))
            } else if let Some(value) = value.as_u64() {
                let value = i64::try_from(value)?;
                Ok(Value::int(value, span))
            } else if let Some(value) = value.as_f64() {
                Ok(Value::float(value, span))
            } else {
                Ok(Value::nothing(span))
            }
        }
        serde_json::Value::String(value) => Ok(Value::string(value, span)),
        serde_json::Value::Array(values) => {
            let values = values
                .into_iter()
                .map(|value| serde_json_to_nu_value(value, span))
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(Value::list(values, span))
        }
        serde_json::Value::Object(object) => {
            let record = object
                .into_iter()
                .map(|(key, value)| {
                    serde_json_to_nu_value(value, span)
                        .map(|value| (key, value))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;

            Ok(Value::record(record.into_iter().collect(), span))
        }
    }
}

fn labeled_error(
    span: Span,
    message: impl Into<String>,
    error: impl std::fmt::Display,
) -> LabeledError {
    LabeledError::new(message.into())
        .with_label(error.to_string(), span)
}

#[test]
fn test_examples() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;

    PluginTest::new("all_querier", AllQuerierPlugin.into())?
        .test_command_examples(&QueryWikidata)
}
use std::{
    collections::HashMap,
    path::PathBuf,
};

use anyhow::Context;
use clap::{
    Parser,
    Subcommand,
};
use serde_json::{
    Map,
    Value,
};

use allq_providers::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderPageData,
    resolve_provider_link,
};
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "allq")]
#[command(about = "Query all the things")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Fetch and print one Wikidata entity by QID
    EntityByQid {
        #[arg(short, long)]
        qid: String,

        #[arg(long, help = "Only read from the local Wikidata cache; do not call the Wikidata API")]
        cache_only: bool,

        #[arg(long, conflicts_with = "cache_only", help = "Ignore cached entity data and fetch from Wikidata")]
        force_fetch: bool,
    },

    /// Fetch a machine-readable JSON version of Wikidata's property list
    BootstrapProperties {
        #[arg(short, long)]
        out: PathBuf,
    },

    /// List Wikidata properties
    ListProperties {
        /// Output compact JSON for shell consumers such as nushell
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON
        #[arg(long, conflicts_with = "json")]
        pretty: bool,

        /// Fetch current data from Wikidata instead of using the built-in snapshot
        #[arg(long)]
        refresh: bool,
    },

    /// List curated Wikidata item types/classes
    ListTypes {
        /// Output compact JSON for shell consumers such as nushell
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON
        #[arg(long, conflicts_with = "json")]
        pretty: bool,
    },

    /// Search Wikidata items by query/title and instance-of type
    SearchItem {
        /// Curated type key or label, e.g. anime-tv-series, film, video game, character
        #[arg(short = 't', long = "type", conflicts_with = "type_qid")]
        item_type: Option<String>,

        /// Raw Wikidata class QID, e.g. Q63952888
        #[arg(long)]
        type_qid: Option<String>,

        /// Search query/title, e.g. Bleach
        #[arg(short, long)]
        query: String,

        /// Follow a supported external link from the resolved Wikidata item, e.g. waifu
        #[arg(short = 'l', long = "link")]
        link: Option<String>,

        /// Maximum number of output results, clamped to 1..=50
        #[arg(short, long, default_value_t = 1)]
        limit: usize,

        /// Number of raw Wikidata text-search candidates to inspect before type filtering
        #[arg(long)]
        candidate_limit: Option<usize>,

        /// Print the generated SPARQL query and result count to stderr
        #[arg(long)]
        debug_query: bool,

        /// Only read from the local Wikidata cache; do not call the Wikidata API
        #[arg(long, conflicts_with = "force_fetch")]
        cache_only: bool,

        /// Ignore cached search results and fetch from Wikidata
        #[arg(long, conflicts_with = "cache_only")]
        force_fetch: bool,

        /// Only match direct P31 values; do not include subclasses via P279
        #[arg(long)]
        direct_only: bool,

        /// Add Wikidata property-name annotations to hydrated JSON entity output
        #[arg(long)]
        annotate_properties: bool,

        /// Output compact JSON for shell consumers such as nushell
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON
        #[arg(long, conflicts_with = "json")]
        pretty: bool,
    },
    /// Print normalized Wikidata external IDs for one entity
    EntityIds {
        #[arg(short, long)]
        qid: String,
        #[arg(long)]
        cache_only: bool,
        #[arg(long, conflicts_with = "cache_only")]
        force_fetch: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, conflicts_with = "json")]
        pretty: bool,
    },
}

impl Cli {
    fn debug_logging_enabled(&self) -> bool {
        match &self.command {
            Command::SearchItem { debug_query, .. } => *debug_query,
            _ => false,
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = try_main().await {
        eprintln!("error: {error:#}");

        let mut source = error.source();
        while let Some(error) = source {
            eprintln!("caused by: {error}");
            source = error.source();
        }

        std::process::exit(1);
    }
}

async fn try_main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_logging(cli.debug_logging_enabled())?;

    match cli.command {
        Command::EntityByQid {
            qid,
            cache_only,
            force_fetch,
        } => {
            allq_wikidata::retrieve_entity_by_qid_with_options(
                &qid,
                cache_only,
                force_fetch,
            )
                .await?;
        }
        Command::BootstrapProperties { out } => {
            let rows = allq_wikidata::fetch_listproperties_rows_json().await?;
            let json = serde_json::to_string_pretty(&rows)?;
            tokio::fs::write(out, json).await?;
        }
        Command::ListProperties {
            json,
            pretty,
            refresh,
        } => {
            let rows = allq_wikidata::list_properties_id_name_description_json(refresh).await?;

            if json {
                println!("{}", serde_json::to_string(&rows)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                println!("id\tname\tdescription");

                for row in rows {
                    println!(
                        "{}\t{}\t{}",
                        clean_tsv_field(&row.id),
                        clean_tsv_field(&row.name),
                        clean_tsv_field(row.description.as_deref().unwrap_or(""))
                    );
                }
            }
        }
        Command::ListTypes { json, pretty } => {
            let rows = allq_wikidata::curated_wikidata_item_types();

            if json {
                println!("{}", serde_json::to_string(rows)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(rows)?);
            } else {
                println!("key\tqid\tlabel\tdescription");

                for row in rows {
                    println!(
                        "{}\t{}\t{}\t{}",
                        clean_tsv_field(row.key),
                        clean_tsv_field(row.qid),
                        clean_tsv_field(row.label),
                        clean_tsv_field(row.description)
                    );
                }
            }
        }
        Command::SearchItem {
            item_type,
            type_qid,
            query,
            link,
            limit,
            candidate_limit,
            debug_query,
            cache_only,
            force_fetch,
            direct_only,
            annotate_properties,
            json,
            pretty,
        } => {
            let type_qid = match (item_type, type_qid) {
                (Some(item_type), None) => {
                    allq_wikidata::resolve_wikidata_item_type_qid(&item_type)?
                }
                (None, Some(type_qid)) => {
                    allq_wikidata::resolve_wikidata_item_type_qid(&type_qid)?
                }
                (None, None) => {
                    anyhow::bail!("provide either --type or --type-qid");
                }
                (Some(_), Some(_)) => {
                    anyhow::bail!("provide only one of --type or --type-qid");
                }
            };

            let rows = allq_wikidata::search_items_by_instance_of_with_options(
                &type_qid,
                &query,
                allq_wikidata::SearchItemsByInstanceOfOptions {
                    output_limit: Some(limit),
                    candidate_limit,
                    include_subclasses: !direct_only,
                    debug_query,
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

            if debug_query {
                eprintln!(
                    "debug: hydrating {} search-item result(s) via entity-by-qid",
                    rows.len()
                );
                eprintln!("debug: entity lookup mode={lookup_mode:?}");
            }

            let client = if cache_only {
                allq_wikidata::WikidataClient::new_local_only().await?
            } else {
                allq_wikidata::WikidataClient::new().await?
            };

            let mut entities = hydrate_search_item_entities(&client, &rows, lookup_mode).await?;

            if let Some(link) = link {
                let provider_http_client = ProviderHttpClient::new()?;
                let followed = follow_search_item_link(
                    &client,
                    &provider_http_client,
                    &entities,
                    lookup_mode,
                    &link,
                )
                    .await?;

                if json {
                    println!("{}", serde_json::to_string(&followed)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&followed)?);
                }

                return Ok(());
            }

            allq_wikidata::add_external_links_to_entities(
                &mut entities,
                &client,
                lookup_mode,
            )
                .await?;

            if annotate_properties {
                let property_names = wikidata_property_names_by_id().await?;
                annotate_entities_with_property_names(&mut entities, &property_names);
            }

            if json {
                println!("{}", serde_json::to_string(&entities)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(&entities)?);
            } else {
                println!("id\tlabel\tdescription");

                for row in rows {
                    println!(
                        "{}\t{}\t{}",
                        clean_tsv_field(&row.id),
                        clean_tsv_field(&row.label),
                        clean_tsv_field(row.description.as_deref().unwrap_or(""))
                    );
                }
            }
        },
        Command::EntityIds { qid, cache_only, force_fetch, json, pretty } => {
            let mode = if cache_only { allq_wikidata::WikidataEntityLookupMode::CacheOnly } else if force_fetch { allq_wikidata::WikidataEntityLookupMode::ForceFetch } else { allq_wikidata::WikidataEntityLookupMode::NetworkFallback };
            let client = if cache_only { allq_wikidata::WikidataClient::new_local_only().await? } else { allq_wikidata::WikidataClient::new().await? };
            let ids = allq_wikidata::external_ids_by_qid(&qid, &client, mode).await?;
            if json { println!("{}", serde_json::to_string(&ids)?); }
            else if pretty { println!("{}", serde_json::to_string_pretty(&ids)?); }
            else { print_external_ids_tsv(&ids); }
        }
    }

    Ok(())
}

async fn hydrate_search_item_entities(
    client: &allq_wikidata::WikidataClient,
    rows: &[allq_wikidata::WikidataItemSearchResult],
    lookup_mode: allq_wikidata::WikidataEntityLookupMode,
) -> anyhow::Result<Vec<Value>> {
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

fn provider_page_cache_key(source: &str, value: &str) -> String {
    format!("external_page:{source}:{value}")
}

fn normalize_link_key(link: &str) -> String {
    link.trim().to_ascii_lowercase()
}

fn init_logging(debug_logging: bool) -> anyhow::Result<()> {
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::try_new(rust_log)
            .context("invalid RUST_LOG env filter")?
    } else if debug_logging {
        EnvFilter::try_new("warn,allq_cli=debug,allq_providers=debug,allq_wikidata=debug")
            .context("invalid built-in debug env filter")?
    } else {
        EnvFilter::try_new("warn")
            .context("invalid built-in default env filter")?
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init();

    Ok(())
}
async fn wikidata_property_names_by_id() -> anyhow::Result<HashMap<String, String>> {
    let properties = allq_wikidata::list_properties_id_name_description_json(false).await?;

    Ok(properties
        .into_iter()
        .map(|property| (property.id, property.name))
        .collect())
}

fn annotate_entities_with_property_names(
    entities: &mut [Value],
    property_names: &HashMap<String, String>,
) {
    for entity in entities {
        add_entity_claim_property_names(entity, property_names);
        annotate_value_property_names(entity, property_names);
    }
}

fn add_entity_claim_property_names(
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

fn annotate_value_property_names(
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

fn print_external_ids_tsv(external_ids: &[allq_wikidata::ExternalId]) {
    println!("wikidataQid\tpropertyId\tpropertyName\tvalue\tsource\tsupported\turls");

    for external_id in external_ids {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            clean_tsv_field(external_id.wikidata_qid.as_deref().unwrap_or("")),
            clean_tsv_field(&external_id.property_id),
            clean_tsv_field(external_id.property_name.as_deref().unwrap_or("")),
            clean_tsv_field(&external_id.value),
            clean_tsv_field(external_id.source.as_deref().unwrap_or("")),
            external_id.supported,
            clean_tsv_field(&external_id.urls.join(" "))
        );
    }
}

fn clean_tsv_field(value: &str) -> String {
    value
        .replace('\t', " ")
        .replace(['\r', '\n'], " ")
}
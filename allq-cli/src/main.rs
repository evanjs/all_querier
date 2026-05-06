use std::path::PathBuf;

use anyhow::Context;
use clap::{
    Parser,
    Subcommand,
};
use allq_query::{
    WikidataQueryOptions,
    WikidataQueryResult,
    query_wikidata,
};
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
            let result = query_wikidata(WikidataQueryOptions {
                item_type: item_type.as_deref(),
                type_qid: type_qid.as_deref(),
                query: &query,
                link: link.as_deref(),
                limit,
                candidate_limit,
                cache_only,
                force_fetch,
                direct_only,
                debug_query,
                annotate_properties,
            })
                .await?;

            if json {
                println!("{}", serde_json::to_string(&result.to_json_value())?);
            } else if pretty || link.is_some() {
                println!("{}", serde_json::to_string_pretty(&result.to_json_value())?);
            } else {
                match result {
                    WikidataQueryResult::Entities { rows, .. } => {
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
                    WikidataQueryResult::FollowedLink { value } => {
                        println!("{}", serde_json::to_string_pretty(&value)?);
                    }
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
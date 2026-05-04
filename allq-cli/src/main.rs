use std::path::PathBuf;

use clap::{
    Parser,
    Subcommand,
};

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
        /// Curated type key, e.g. anime-tv-series, film, video-game
        #[arg(short = 't', long = "type", conflicts_with = "type_qid")]
        item_type: Option<String>,

        /// Raw Wikidata class QID, e.g. Q63952888
        #[arg(long)]
        type_qid: Option<String>,

        /// Search query/title, e.g. Bleach
        #[arg(short, long)]
        query: String,

        /// Maximum number of output results, clamped to 1..=50
        #[arg(short, long, default_value_t = 1)]
        limit: usize,

        /// Number of raw Wikidata text-search candidates to inspect before type filtering
        #[arg(long)]
        candidate_limit: Option<usize>,

        /// Print the generated SPARQL query and result count to stderr
        #[arg(long)]
        debug_query: bool,

        /// Only match direct P31 values; do not include subclasses via P279
        #[arg(long)]
        direct_only: bool,

        /// Output compact JSON for shell consumers such as nushell
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON
        #[arg(long, conflicts_with = "json")]
        pretty: bool,
    },
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
            limit,
            candidate_limit,
            debug_query,
            direct_only,
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
                },
            )
                .await?;

            if json {
                println!("{}", serde_json::to_string(&rows)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(&rows)?);
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
        }
    }

    Ok(())
}

fn clean_tsv_field(value: &str) -> String {
    value
        .replace('\t', " ")
        .replace(['\r', '\n'], " ")
}
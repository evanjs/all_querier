use std::path::PathBuf;
use std::time::Duration;
use anyhow::Context;
use clap::{
    Parser,
    Subcommand,
};
use serde_json::to_string;
use tracing::{debug, warn};
use allq_core::{FetchMode, SearchDispatcher, SearchOptions, SearchResult};
use allq_query::{
    FetchArgs,
    WikidataQueryOptions,
    WikidataQueryResult,
    query_wikidata,
};
use allq_musicbrainz::MusicBrainzSearchProvider;
use allq_pcgw::PcgwSearchProvider;
use allq_wikidata::WikidataSearchProvider;
use tracing_subscriber::EnvFilter;
use allq_anilist::AniListProvider;
use allq_jikan::JikanProvider;

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

        #[command(flatten)]
        fetch: FetchArgs,
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
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Number of raw Wikidata text-search candidates to inspect before type filtering
        #[arg(long)]
        candidate_limit: Option<usize>,

        /// Print the generated SPARQL query and result count to stderr
        #[arg(long)]
        debug_query: bool,

        #[command(flatten)]
        fetch: FetchArgs,

        /// Add computed externalLinks metadata to hydrated JSON entity output
        #[arg(long)]
        external_links: bool,

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
    /// Search across multiple providers (MusicBrainz, Wikidata) for items by type
    Search {
        /// Free-text search query, e.g. 'OK Computer'. Optional for `animelist`/`mangalist`.
        #[arg()]
        query: Option<String>,

        /// Item type to search for (e.g. album, artist, song, character, video-game)
        #[arg(short = 't', long = "type")]
        item_type: Option<String>,

        /// Restrict search to a single provider (e.g. musicbrainz, wikidata)
        #[arg(short = 'p', long = "provider")]
        provider: Option<String>,

        /// Maximum number of results per provider
        #[arg(short = 'n', long)]
        limit: Option<u32>,

        #[command(flatten)]
        fetch: FetchArgs,

        /// Output compact JSON for shell consumers such as nushell
        #[arg(long)]
        json: bool,

        /// Pretty-print JSON
        #[arg(long, conflicts_with = "json")]
        pretty: bool,

        /// Filter MAL results by media sub-type (e.g. tv, ova, movie, manga, novel)
        #[arg(short = 'm', long = "media-type")]
        media_type: Option<String>,

        /// Use a specific MyAnimeList username for `animelist`/`mangalist` searches
        #[arg(long = "mal-username")]
        mal_username: Option<String>,

        #[arg(long = "anilist-username")]
        anilist_username: Option<String>,

        /// Include NSFW results in MAL searches
        #[arg(long)]
        nsfw: bool,

        /// Enable verbose diagnostic logging to stderr
        #[arg(short, long)]
        verbose: bool,
    },

    /// List supported external provider links (e.g. pcgw, waifu)
    ListProviders {
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
        #[command(flatten)]
        fetch: FetchArgs,
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
            Command::Search { verbose, .. } => *verbose,
            _ => false,
        }
    }
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

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
        Command::EntityByQid { qid, fetch } => {
            allq_wikidata::retrieve_entity_by_qid_with_options(
                &qid,
                fetch.cache_only,
                fetch.force_fetch,
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
            fetch,
            external_links,
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
                cache_only: fetch.cache_only,
                force_fetch: fetch.force_fetch,
                direct_only: fetch.direct_only,
                debug_query,
                annotate_properties,
                enrich_external_links: external_links,
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
        Command::Search {
            query,
            item_type,
            provider,
            limit,
            fetch,
            json,
            pretty,
            media_type,
            mal_username,
            anilist_username,
            nsfw,
            verbose: _,
        } => {
            let fetch_mode = if fetch.cache_only {
                FetchMode::CacheOnly
            } else if fetch.force_fetch {
                FetchMode::ForceFetch
            } else {
                FetchMode::NetworkFallback
            };
            let query = match (query, item_type.as_deref()) {
                (Some(query), _) => query,
                (None, Some("animelist" | "mangalist")) => String::new(),
                (None, Some(item_type)) => anyhow::bail!(
                    "search query is required unless --type is animelist or mangalist (got {item_type})"
                ),
                (None, None) => {
                    anyhow::bail!(
                        "search query is required unless --type is animelist or mangalist"
                    )
                }
            };
            let results = run_search(
                &query,
                item_type.as_deref(),
                provider.as_deref(),
                limit,
                fetch_mode,
                media_type.as_deref(),
                mal_username.as_deref(),
                anilist_username.as_deref(),
                nsfw,
            )
            .await?;

            if json {
                println!("{}", serde_json::to_string(&results)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                println!("provider\tid\tlabel\tdescription\titem_type");
                for r in &results {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        clean_tsv_field(&r.provider),
                        clean_tsv_field(&r.id),
                        clean_tsv_field(&r.label),
                        clean_tsv_field(r.description.as_deref().unwrap_or("")),
                        clean_tsv_field(r.item_type.as_deref().unwrap_or(""))
                    );
                }
            }
        }
        Command::ListProviders { json, pretty } => {
            let links = allq_providers::supported_provider_links();

            if json {
                println!("{}", serde_json::to_string(links)?);
            } else if pretty {
                println!("{}", serde_json::to_string_pretty(links)?);
            } else {
                println!("primaryAlias\tsource\tpropertyId\tsupportedItemTypes\tdescription");

                for link in links {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        clean_tsv_field(link.primary_alias),
                        clean_tsv_field(link.source),
                        clean_tsv_field(link.property_id),
                        clean_tsv_field(&link.supported_item_types.join(", ")),
                        clean_tsv_field(link.description),
                    );
                }
            }
        }
        Command::EntityIds { qid, fetch, json, pretty } => {
            let mode = if fetch.cache_only { allq_wikidata::WikidataEntityLookupMode::CacheOnly } else if fetch.force_fetch { allq_wikidata::WikidataEntityLookupMode::ForceFetch } else { allq_wikidata::WikidataEntityLookupMode::NetworkFallback };
            let client = if fetch.cache_only { allq_wikidata::WikidataClient::new_local_only().await? } else { allq_wikidata::WikidataClient::new().await? };
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

fn user_agent_email() -> String {
    let author = env!("CARGO_PKG_AUTHORS")
        .split(':')
        .next()
        .unwrap_or(env!("CARGO_PKG_AUTHORS"));
    let email = author
        .split_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map(|(email, _)| email.trim())
        .unwrap_or(author.trim());
    format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        email,
    )
}

async fn run_search(
    query: &str,
    item_type: Option<&str>,
    provider_filter: Option<&str>,
    limit: Option<u32>,
    fetch_mode: FetchMode,
    media_type: Option<&str>,
    mal_username: Option<&str>,
    anilist_username: Option<&str>,
    nsfw: bool,
) -> anyhow::Result<Vec<SearchResult>> {
    let mut dispatcher = SearchDispatcher::new();
    //let mut caches = Vec::new();

    let should_add = |name: &str| provider_filter.map_or(true, |f| f == name);

    if should_add("musicbrainz") {
        let cache = allq_core::create_provider_cache("musicbrainz").await?;
        //caches.push(cache.clone());
        dispatcher.add_provider(Box::new(MusicBrainzSearchProvider::new_with_cache(&user_agent_email(), cache)));
    }

    if should_add("wikidata") {
        let client = allq_wikidata::WikidataClient::new().await?;
        dispatcher.add_provider(Box::new(WikidataSearchProvider::new(client)));
    }

    if should_add("pcgw") {
        let cache = allq_core::create_provider_cache("pcgw").await?;
        //caches.push(cache.clone());
        dispatcher.add_provider(Box::new(PcgwSearchProvider::new_with_cache(&user_agent_email(), cache)));
    }

    if should_add("jikan") {
        let cache = allq_core::create_provider_cache("jikan").await?;
        //caches.push(cache.clone());
        dispatcher.add_provider(Box::new(JikanProvider::new_with_cache(cache)))
    }

    if should_add("anilist") {
        let cache = allq_core::create_provider_cache("anilist").await?;
        //caches.push(cache.clone());
        dispatcher.add_provider(Box::new(AniListProvider::new_with_cache(cache)));
    }

    if should_add("myanimelist") {
        let _cache = allq_core::create_provider_cache("myanimelist").await?;
        match allq_mal::MalProvider::new() {
            Ok(mal_provider) => {
                dispatcher.add_provider(Box::new(mal_provider));
            }
            Err(e) => {
                tracing::warn!("Failed to initialize MyAnimeList provider: {e}");
            }
        }
    }

    if dispatcher.provider_names().is_empty() {
        anyhow::bail!(
            "no providers match filter {:?}. Available: musicbrainz, wikidata, pcgw, myanimelist",
            provider_filter
        );
    }

    let options = SearchOptions {
        limit,
        language: Some("en".to_string()),
        fetch_mode,
        media_type: media_type.map(|s| s.to_string()),
        mal_username: mal_username.map(|s| s.to_string()),
        anilist_username: anilist_username.map(|s|s.to_string()),
        nsfw,
    };

    let results = dispatcher.search(query, item_type, &options).await?;
    //tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    debug!("search returned; dropping dispatcher");
    drop(dispatcher);
    debug!("dispatcher dropped; waiting for caches");

    // for cache in caches {
    //     match tokio::time::timeout(Duration::from_secs(2), cache.storage().wait()).await {
    //         Ok(()) => debug!("cache storage queue drained"),
    //         Err(_) => warn!("timed out waiting for cache storage queue to drain"),
    //     }
    // }

    Ok(results)
}

fn init_logging(debug_logging: bool) -> anyhow::Result<()> {
    let env_filter = if let Ok(rust_log) = std::env::var("RUST_LOG") {
        EnvFilter::try_new(rust_log)
            .context("invalid RUST_LOG env filter")?
    } else if debug_logging {
        EnvFilter::try_new("warn,allq_cli=debug,allq_providers=debug,allq_wikidata=debug,allq_core=debug,allq_mal=debug,allq_pcgw=debug")
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
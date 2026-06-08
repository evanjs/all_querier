use std::convert::Infallible;
use std::sync::OnceLock;
use std::time::Duration;
use strum::VariantNames;
use tracing::{debug, error, warn};

use crate::{
    AllQuerierPlugin, init_logging, labeled_error, serde_json_to_nu_value, user_agent_email,
};
use allq_core::{FetchMode, GameSearchOptions, GameStoreType, SearchDispatcher, SearchOptions};
use allq_igdb::{IGDBProvider, SUPPORTED_TYPES as IGDB_SUPPORTED_TYPES};
use allq_pcgw::{PcgwSearchProvider, SUPPORTED_TYPES as PCGW_SUPPORTED_TYPES};
use allq_rawg::{RawgProvider, SUPPORTED_TYPES as RAWG_SUPPORTED_TYPES};
use allq_query::{add_fetch_flags, read_fetch_args};
use allq_wikidata::{CURATED_WIKIDATA_ITEM_TYPE_KEYS, WikidataSearchProvider};
use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Completion, Example, Flag, FromValue, LabeledError, Signature, Span, SyntaxShape,
    Value,
};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
fn runtime() -> anyhow::Result<&'static tokio::runtime::Runtime> {
    Ok(RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create Tokio runtime")
    }))
}

/// Static list of provider names supported by the `search` command.
pub const SEARCH_PROVIDER_NAMES: &[&str] = &[
    "wikidata",
    "pcgw",
    "rawg",
    "igdb"
];

/// Returns the union of item types supported across all search providers,
/// suitable for use as completion candidates for the `--type` flag.
fn search_item_type_completions() -> &'static [&'static str] {
    static CACHED: OnceLock<Vec<&'static str>> = OnceLock::new();
    CACHED.get_or_init(|| {
        let mut types: Vec<&'static str> = Vec::new();
        for &t in CURATED_WIKIDATA_ITEM_TYPE_KEYS {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        for &t in PCGW_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        for &t in RAWG_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        for &t in IGDB_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        types
    })
}

pub struct SearchGame;

impl SimplePluginCommand for SearchGame {
    type Plugin = AllQuerierPlugin;

    fn name(&self) -> &str {
        "search game"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build(self.name())
            .optional(
                "query",
                SyntaxShape::String,
                "Free-text search query, e.g. 'Book of Hours'",
            )
            .param(
                Flag::new("provider")
                    .short('p')
                    .arg(SyntaxShape::String)
                    .desc("Restrict search to a single provider (e.g. PCGW, IGDB, RAWG)")
                    .completion(Completion::new_list(SEARCH_PROVIDER_NAMES)),
            )
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of results per provider",
                Some('n'),
            )
            // .switch(
            //     "nsfw",
            //     "Include NSFW results in searches",
            //     None,
            // )
            .param(
                Flag::new("provider-direct-id-search")
                    .short('S')
                    .arg(SyntaxShape::String)
                    .desc("For compatible providers, searches using the provided Store's ID instead of a search query")
                    .completion(Completion::new_list(GameStoreType::VARIANTS))
            );
        add_fetch_flags(sig)
            .switch(
                "verbose",
                "Enable verbose diagnostic logging to stderr",
                Some('v'),
            )
            .category(Category::Experimental)
    }

    fn description(&self) -> &str {
        "Search across multiple providers (PCGW, IGDB, RAWG) for items by type"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: r#"search "Dredge" --type video-game"#,
                description: "Search for a video game (Wikidata has good game coverage)",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _plugin: &AllQuerierPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let item_type: Option<String> = call.get_flag("type")?;
        let query = match (call.opt::<String>(0)?, item_type.as_deref()) {
            (Some(query), _) => query,
            (None, Some(item_type)) => {
                return Err(LabeledError::new("search query is required".to_string())
                .with_label("missing query", call.head));
            }
            (None, None) => {
                return Err(LabeledError::new("search query is required", )
                .with_label("missing query", call.head));
            }
        };
        let provider: Option<String> = call.get_flag("provider")?;
        let limit = call
            .get_flag::<i64>("limit")?
            .and_then(|l| u32::try_from(l).ok());
        let fetch = read_fetch_args(call).map_err(|e| e)?;
        // let nsfw = call.has_flag("nsfw")?;
        let provider_direct_id_search: Option<String> =
            call.get_flag("provider-direct-id-search")?;
        debug!(?provider_direct_id_search);
        let provider_direct_id_search = match provider_direct_id_search {
            Some(game_store_type) => {
                debug!(?game_store_type);
                match GameStoreType::try_from(game_store_type.as_str()) {
                    Ok(valid_game_store_type) => {
                        debug!(?valid_game_store_type);
                        Some(valid_game_store_type)
                    }
                    Err(error) => {
                        error!(
                            ?error,
                            "Failed to parse GameStoreType! Not using direct ID search"
                        );
                        None
                    }
                }
            }
            _ => None,
        };

        let verbose = call.has_flag("verbose")?;
        let head = call.head;

        init_logging(verbose)
            .map_err(|e| labeled_error(head, "Failed to initialize logging", e))?;

        let fetch_mode = if fetch.cache_only {
            FetchMode::CacheOnly
        } else if fetch.force_fetch {
            FetchMode::ForceFetch
        } else {
            FetchMode::NetworkFallback
        };

        let results = runtime()
            .map_err(|e| labeled_error(head, "Failed to create Tokio runtime", e))?
            .block_on(run_search(
                &query,
                provider.as_deref(),
                limit,
                fetch_mode,
                // nsfw,
                provider_direct_id_search,
            ))
            .map_err(|e| labeled_error(head, "Search failed", e))?;

        let json = serde_json::to_value(&results)
            .map_err(|e| labeled_error(head, "Failed to serialize results", e))?;

        serde_json_to_nu_value(json, head)
            .map_err(|e| labeled_error(head, "Failed to convert results to Nushell value", e))
    }
}

async fn run_search(
    query: &str,
    provider_filter: Option<&str>,
    limit: Option<u32>,
    fetch_mode: FetchMode,
    // nsfw: bool,
    provider_direct_id_search: Option<GameStoreType>,
) -> anyhow::Result<Vec<allq_core::SearchResult>> {
    let mut dispatcher = SearchDispatcher::new();

    let should_add = |name: &str| provider_filter.map_or(true, |f| f == name);
    if should_add("wikidata") {
        let client = allq_wikidata::WikidataClient::new().await?;
        dispatcher.add_provider(Box::new(WikidataSearchProvider::new(client)));
    }

    if should_add("pcgw") {
        let cache = allq_core::create_provider_cache("pcgw").await?;
        dispatcher.add_provider(Box::new(PcgwSearchProvider::new_with_cache(
            &user_agent_email(),
            cache,
        )));
    }

    if should_add("rawg") {
        let cache = allq_core::create_provider_cache("rawg").await?;
        match RawgProvider::new_with_cache(cache) {
            Ok(rawg_provider) => {
                dispatcher.add_provider(Box::new(rawg_provider));
            }
            Err(e) => {
                debug!("Failed to initialize RAWG provider, skipping: {}", e);
            }
        }
    }

    if should_add("igdb") {
        let cache = allq_core::create_provider_cache("igdb").await?;
        match IGDBProvider::new_with_cache(cache) {
            Ok(igdb_provider) => {
                dispatcher.add_provider(Box::new(igdb_provider));
            }
            Err(e) => {
                debug!("Failed to initialize IGDB provider, skipping: {}", e);
            }
        }
    }

    if dispatcher.provider_names().is_empty() {
        anyhow::bail!(
            "no providers match filter {:?}. Available: pcgw, igdb, rawg, wikidata",
            provider_filter
        );
    }

    let options = GameSearchOptions {
        limit,
        language: Some("en".to_string()),
        fetch_mode,
        // nsfw,
        provider_direct_id_search,
    };

    dispatcher.search_games(query, &options).await
}

use std::convert::Infallible;
use std::sync::OnceLock;
use std::time::Duration;
use strum::VariantNames;
use tracing::{debug, error, warn};

use crate::{
    AllQuerierPlugin, init_logging, labeled_error, serde_json_to_nu_value, user_agent_email,
};
use allq_anilist::{AniListProvider, SUPPORTED_TYPES as ANILIST_SUPPORTED_TYPES};
use allq_core::{FetchMode, GameStoreType, SearchDispatcher, SearchOptions};
use allq_igdb::{IGDBProvider, SUPPORTED_TYPES as IGDB_SUPPORTED_TYPES};
use allq_itis::{ItisProvider, SUPPORTED_TYPES as ITIS_SUPPORTED_TYPES};
use allq_jikan::JikanProvider;
use allq_mal::{MAL_MEDIA_TYPES, SUPPORTED_TYPES as MAL_SUPPORTED_TYPES};
use allq_musicbrainz::{MusicBrainzSearchProvider, SUPPORTED_TYPES as MUSICBRAINZ_SUPPORTED_TYPES};
use allq_pcgw::{PcgwSearchProvider, SUPPORTED_TYPES as PCGW_SUPPORTED_TYPES};
use allq_query::{add_fetch_flags, read_fetch_args};
use allq_rawg::{RawgProvider, SUPPORTED_TYPES as RAWG_SUPPORTED_TYPES};
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
    "musicbrainz",
    "wikidata",
    "pcgw",
    "myanimelist",
    "jikan",
    "anilist",
    "itis",
    "rawg",
    "igdb",
];
pub const GAME_STORE_TYPES: &[&str] = {
    let variants = GameStoreType::VARIANTS;
    variants
};

/// Returns the union of item types supported across all search providers,
/// suitable for use as completion candidates for the `--type` flag.
fn search_item_type_completions() -> &'static [&'static str] {
    static CACHED: OnceLock<Vec<&'static str>> = OnceLock::new();
    CACHED.get_or_init(|| {
        let mut types: Vec<&'static str> = Vec::new();
        for &t in MUSICBRAINZ_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
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
        for &t in MAL_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        for &t in ANILIST_SUPPORTED_TYPES {
            if !types.contains(&t) {
                types.push(t);
            }
        }
        for &t in ITIS_SUPPORTED_TYPES {
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

pub struct Search;

impl SimplePluginCommand for Search {
    type Plugin = AllQuerierPlugin;

    fn name(&self) -> &str {
        "search"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build(self.name())
            .optional(
                "query",
                SyntaxShape::String,
                "Free-text search query, e.g. 'OK Computer'. Optional for animelist/mangalist.",
            )
            .param(
                Flag::new("type")
                    .short('t')
                    .arg(SyntaxShape::String)
                    .desc("Item type to search for (e.g. album, artist, song, character, video-game)")
                    .completion(Completion::new_list(search_item_type_completions())),
            )
            .param(
                Flag::new("provider")
                    .short('p')
                    .arg(SyntaxShape::String)
                    .desc("Restrict search to a single provider (e.g. musicbrainz, wikidata)")
                    .completion(Completion::new_list(SEARCH_PROVIDER_NAMES)),
            )
            .named(
                "limit",
                SyntaxShape::Int,
                "Maximum number of results per provider",
                Some('n'),
            )
            .param(
                Flag::new("media-type")
                    .short('m')
                    .arg(SyntaxShape::String)
                    .desc("Filter MAL results by media sub-type (e.g. tv, ova, movie, manga, novel)")
                    .completion(Completion::new_list(MAL_MEDIA_TYPES)),
            )
            .param(
                Flag::new("mal-username")
                    .arg(SyntaxShape::String)
                    .desc("Use a specific MyAnimeList username for animelist or mangalist searches"),
            )
            .param(
                Flag::new("anilist-username")
                    .arg(SyntaxShape::String)
                    .desc("Use a specific AniList username for animelist or mangalist searches"),
            )
            .switch(
                "nsfw",
                "Include NSFW results in MAL searches",
                None,
            )
            .param(
                Flag::new("provider-direct-id-search")
                    .short('S')
                    .arg(SyntaxShape::String)
                    .desc("For compatible providers, searches using the provided Store's ID instead of a search query")
                    .completion(Completion::new_list(GAME_STORE_TYPES))
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
        "Search across multiple providers (MusicBrainz, Wikidata) for items by type"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: r#"search "...Like Clockwork" --type album"#,
                description: "Search for an album across MusicBrainz and Wikidata",
                result: None,
            },
            Example {
                example: r#"search "Queens of the Stone Ago" --type artist --provider musicbrainz"#,
                description: "Search for an artist on MusicBrainz only",
                result: None,
            },
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
            // Search query is not required for animelist nor mangalist
            (None, Some("animelist" | "mangalist")) => String::new(),
            (None, Some(item_type)) => {
                return Err(LabeledError::new(format!(
                    "search query is required unless --type is animelist or mangalist (got {item_type})"
                ))
                .with_label("missing query", call.head));
            }
            (None, None) => {
                return Err(LabeledError::new(
                    "search query is required unless --type is animelist or mangalist",
                )
                .with_label("missing query", call.head));
            }
        };
        let provider: Option<String> = call.get_flag("provider")?;
        let limit = call
            .get_flag::<i64>("limit")?
            .and_then(|l| u32::try_from(l).ok());
        let fetch = read_fetch_args(call).map_err(|e| e)?;
        let media_type: Option<String> = call.get_flag("media-type")?;
        let mal_username: Option<String> = call.get_flag("mal-username")?;
        let anilist_username: Option<String> = call.get_flag("anilist-username")?;
        let nsfw = call.has_flag("nsfw")?;
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
                item_type.as_deref(),
                provider.as_deref(),
                limit,
                fetch_mode,
                media_type.as_deref(),
                mal_username.as_deref(),
                anilist_username.as_deref(),
                nsfw,
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
    item_type: Option<&str>,
    provider_filter: Option<&str>,
    limit: Option<u32>,
    fetch_mode: FetchMode,
    media_type: Option<&str>,
    mal_username: Option<&str>,
    anilist_username: Option<&str>,
    nsfw: bool,
    provider_direct_id_search: Option<GameStoreType>,
) -> anyhow::Result<Vec<allq_core::SearchResult>> {
    let mut dispatcher = SearchDispatcher::new();

    let should_add = |name: &str| provider_filter.map_or(true, |f| f == name);
    if should_add("musicbrainz") {
        let cache = allq_core::create_provider_cache("musicbrainz").await?;
        dispatcher.add_provider(Box::new(MusicBrainzSearchProvider::new_with_cache(&user_agent_email(), cache)));
    }

    if should_add("wikidata") {
        let client = allq_wikidata::WikidataClient::new().await?;
        dispatcher.add_provider(Box::new(WikidataSearchProvider::new(client)));
    }

    if should_add("pcgw") {
        let cache = allq_core::create_provider_cache("pcgw").await?;
        dispatcher.add_provider(Box::new(PcgwSearchProvider::new_with_cache(&user_agent_email(), cache)));
    }

    if should_add("jikan") {
        let cache = allq_core::create_provider_cache("jikan").await?;
        dispatcher.add_provider(Box::new(JikanProvider::new_with_cache(cache)))
    }

    if should_add("anilist") {
        let cache = allq_core::create_provider_cache("anilist").await?;
        dispatcher.add_provider(Box::new(AniListProvider::new_with_cache(cache)))
    }

    if should_add("myanimelist") {
        let _cache = allq_core::create_provider_cache("myanimelist").await?;
        match allq_mal::MalProvider::new() {
            Ok(mal_provider) => {
                dispatcher.add_provider(Box::new(mal_provider));
            }
            Err(e) => {
                debug!("Failed to initialize MAL provider, skipping: {}", e);
            }
        }
    }

    if should_add("itis") {
        match allq_itis::ItisProvider::new() {
            Ok(itis_provider) => {
                dispatcher.add_provider(Box::new(itis_provider));
            }
            Err(e) => {
                debug!("Failed to initialize ITIS provider, skipping: {}", e);
            }
        }
    }

    if should_add("rawg") {
        let cache = allq_core::create_provider_cache("rawg").await?;
        match RawgProvider::new_with_cache(cache) {
            Ok(rawg_provider) => {
                dispatcher.add_provider(Box::new(rawg_provider));
            },
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
        anilist_username: anilist_username.map(|s| s.to_string()),
        nsfw,
        provider_direct_id_search,
    };

    dispatcher.search(query, item_type, &options).await
}

use std::sync::OnceLock;
use tracing::debug;

use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{
    Category, Completion, Example, Flag, LabeledError, Signature, Span, SyntaxShape, Value,
};

use allq_core::{FetchMode, SearchDispatcher, SearchOptions};
use allq_query::{add_fetch_flags, read_fetch_args};
use allq_mal::{MAL_MEDIA_TYPES, SUPPORTED_TYPES as MAL_SUPPORTED_TYPES};
use allq_musicbrainz::{MusicBrainzSearchProvider, SUPPORTED_TYPES as MUSICBRAINZ_SUPPORTED_TYPES};
use allq_pcgw::{PcgwSearchProvider, SUPPORTED_TYPES as PCGW_SUPPORTED_TYPES};
use allq_wikidata::{CURATED_WIKIDATA_ITEM_TYPE_KEYS, WikidataSearchProvider};

use crate::{AllQuerierPlugin, init_logging, user_agent_email};

/// Static list of provider names supported by the `search` command.
pub const SEARCH_PROVIDER_NAMES: &[&str] = &["musicbrainz", "wikidata", "pcgw", "myanimelist"];

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
            (None, Some("animelist" | "mangalist")) => String::new(),
            (None, Some(item_type)) => {
                return Err(LabeledError::new(format!(
                    "search query is required unless --type is animelist or mangalist (got {item_type})"
                ))
                .with_label("missing query", call.head));
            }
            (None, None) => {
                return Err(
                    LabeledError::new(
                        "search query is required unless --type is animelist or mangalist",
                    )
                    .with_label("missing query", call.head),
                );
            }
        };
        let provider: Option<String> = call.get_flag("provider")?;
        let limit = call
            .get_flag::<i64>("limit")?
            .and_then(|l| u32::try_from(l).ok());
        let fetch = read_fetch_args(call).map_err(|e| e)?;
        let media_type: Option<String> = call.get_flag("media-type")?;
        let mal_username: Option<String> = call.get_flag("mal-username")?;
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

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| labeled_error(head, "Failed to create Tokio runtime", e))?;

        let results = runtime
            .block_on(run_search(
                &query,
                item_type.as_deref(),
                provider.as_deref(),
                limit,
                fetch_mode,
                media_type.as_deref(),
                mal_username.as_deref(),
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
    };

    dispatcher.search(query, item_type, &options).await
}

fn serde_json_to_nu_value(value: serde_json::Value, span: Span) -> anyhow::Result<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::nothing(span)),
        serde_json::Value::Bool(v) => Ok(Value::bool(v, span)),
        serde_json::Value::Number(v) => {
            if let Some(v) = v.as_i64() {
                Ok(Value::int(v, span))
            } else if let Some(v) = v.as_u64() {
                let v = i64::try_from(v)?;
                Ok(Value::int(v, span))
            } else if let Some(v) = v.as_f64() {
                Ok(Value::float(v, span))
            } else {
                Ok(Value::nothing(span))
            }
        }
        serde_json::Value::String(v) => Ok(Value::string(v, span)),
        serde_json::Value::Array(values) => {
            let values = values
                .into_iter()
                .map(|v| serde_json_to_nu_value(v, span))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(Value::list(values, span))
        }
        serde_json::Value::Object(object) => {
            let record = object
                .into_iter()
                .map(|(key, value)| {
                    serde_json_to_nu_value(value, span).map(|v| (key, v))
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

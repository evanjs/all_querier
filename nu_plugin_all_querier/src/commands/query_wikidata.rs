use nu_plugin::{
    EngineInterface,
    EvaluatedCall,
    SimplePluginCommand,
};
use nu_protocol::{Category, Completion, Example, Flag, LabeledError, Signature, Span, SyntaxShape, Value};
use allq_query::{FetchArgs, add_fetch_flags, read_fetch_args};
use crate::{
    AllQuerierPlugin,
    init_logging,
};

pub struct QueryWikidata;

impl SimplePluginCommand for QueryWikidata {
    type Plugin = AllQuerierPlugin;

    fn name(&self) -> &str {
        "query wikidata"
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build(self.name())
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
                            allq_wikidata::CURATED_WIKIDATA_ITEM_TYPE_KEYS
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
            );
        add_fetch_flags(sig)
            .switch(
                "external-links",
                "Add computed externalLinks metadata to hydrated Wikidata entities",
                None,
            )
            .switch(
                "verbose",
                "Enable verbose diagnostic logging to stderr",
                Some('v'),
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
            .and_then(|limit| usize::try_from(limit).ok());

        let fetch = read_fetch_args(call).map_err(|e| e)?;
        let external_links = call.has_flag("external-links")?;
        let verbose = call.has_flag("verbose")?;
        let head = call.head;

        init_logging(verbose)
            .map_err(|error| labeled_error(head, "Failed to initialize logging", error))?;

        if fetch.cache_only && fetch.force_fetch {
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
                fetch,
                external_links,
                verbose,
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
    limit: Option<usize>,
    fetch: FetchArgs,
    external_links: bool,
    verbose: bool,
) -> anyhow::Result<serde_json::Value> {
    let result = allq_query::query_wikidata(allq_query::WikidataQueryOptions {
        item_type: Some(item_type),
        type_qid: None,
        query,
        link,
        limit, // don't set an explicit limit, as something else might set it conditionally
        // e.g. an expanded limit for series -> tv season queries, etc.
        candidate_limit: None,
        cache_only: fetch.cache_only,
        force_fetch: fetch.force_fetch,
        direct_only: fetch.direct_only,
        debug_query: verbose,
        annotate_properties: false,
        enrich_external_links: external_links,
    })
        .await?;

    Ok(result.into_json_value())
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
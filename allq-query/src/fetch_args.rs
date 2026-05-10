/// Shared fetch-mode flags used by both the CLI and the nushell plugin.
///
/// Controls whether results are read from cache, forced from the network,
/// or resolved normally (cache-with-network-fallback).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct FetchArgs {
    /// Only read from the local cache; do not call Wikidata/provider APIs
    #[cfg_attr(feature = "clap", arg(long, conflicts_with = "force_fetch"))]
    pub cache_only: bool,

    /// Ignore cached data and always fetch from Wikidata/provider APIs
    #[cfg_attr(feature = "clap", arg(long, conflicts_with = "cache_only"))]
    pub force_fetch: bool,

    /// Only match direct P31 values; do not include subclasses via P279
    #[cfg_attr(feature = "clap", arg(long))]
    pub direct_only: bool,
}

/// Adds the shared fetch-mode switches (`--cache-only`, `--force-fetch`, `--direct-only`)
/// to a nushell [`Signature`].
///
/// Call this inside `SimplePluginCommand::signature()` to avoid duplicating switch declarations.
#[cfg(feature = "nu")]
pub fn add_fetch_flags(sig: nu_protocol::Signature) -> nu_protocol::Signature {
    sig.switch(
        "cache-only",
        "Only read from the local cache; do not call Wikidata/provider APIs",
        None,
    )
    .switch(
        "force-fetch",
        "Ignore cached data and always fetch from Wikidata/provider APIs",
        None,
    )
    .switch(
        "direct-only",
        "Only match direct P31 values; do not include subclasses via P279",
        None,
    )
}

/// Reads the shared fetch-mode flags from a nushell [`EvaluatedCall`].
///
/// Returns a [`FetchArgs`] populated from `--cache-only`, `--force-fetch`, and `--direct-only`.
#[cfg(feature = "nu")]
pub fn read_fetch_args(
    call: &nu_plugin::EvaluatedCall,
) -> Result<FetchArgs, nu_protocol::LabeledError> {
    Ok(FetchArgs {
        cache_only: call.has_flag("cache-only")?,
        force_fetch: call.has_flag("force-fetch")?,
        direct_only: call.has_flag("direct-only")?,
    })
}

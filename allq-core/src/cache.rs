use std::path::PathBuf;

use foyer::{BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCacheBuilder, HybridCachePolicy, RecoverMode};

/// Generic string-keyed, string-valued hybrid (memory + disk) cache for provider responses.
pub type ProviderCache = HybridCache<String, String>;

const PROVIDER_CACHE_SCHEMA_VERSION: &str = "v1";
const PROVIDER_MEMORY_CACHE_CAPACITY: usize = 32 * 1024 * 1024;
const PROVIDER_DISK_CACHE_CAPACITY_BYTES: usize = 256 * 1024 * 1024;

fn all_querier_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("all_querier")
}

/// Create a foyer hybrid cache for the given provider name (e.g. `"musicbrainz"`, `"pcgw"`).
///
/// Each provider gets its own isolated cache directory under
/// `$XDG_DATA_HOME/all_querier/foyer/<name>/v1/`.
pub async fn create_provider_cache(name: &str) -> anyhow::Result<ProviderCache> {
    let cache_dir = all_querier_data_dir()
        .join("foyer")
        .join(name)
        .join(PROVIDER_CACHE_SCHEMA_VERSION);

    let device = FsDeviceBuilder::new(cache_dir)
        .with_capacity(PROVIDER_DISK_CACHE_CAPACITY_BYTES)
        .build()?;

    let cache = HybridCacheBuilder::new()
        .with_name(format!("allq {name} cache"))
        .with_policy(HybridCachePolicy::WriteOnInsertion)
        .memory(PROVIDER_MEMORY_CACHE_CAPACITY)
        .storage()
        .with_recover_mode(RecoverMode::Quiet)
        .with_engine_config(BlockEngineConfig::new(device).with_tombstone_log(true))
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create {name} cache: {}", e))?;

    Ok(cache)
}

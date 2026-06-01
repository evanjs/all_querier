use std::path::PathBuf;

use foyer::{BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCacheBuilder, HybridCachePolicy, RecoverMode};
use crate::all_querier_data_dir;

/// Generic string-keyed, string-valued hybrid (memory + disk) cache for provider responses.
pub type ProviderCache = HybridCache<String, String>;

const PROVIDER_CACHE_SCHEMA_VERSION: &str = "v1";
const PROVIDER_MEMORY_CACHE_CAPACITY: usize = 32 * 1024 * 1024;
const PROVIDER_DISK_CACHE_CAPACITY_BYTES: usize = 128 * 1024 * 1024;

pub fn all_querier_cache_dir() -> PathBuf {
    all_querier_data_dir()
        // operate on the directory under the operating system's data directory
        // if it does not exist, operate under a local ".cache" directory
        .unwrap_or_else(|| PathBuf::from(".cache"))
}

/// Create a foyer hybrid cache for the given provider name (e.g. `"musicbrainz"`, `"pcgw"`).
///
/// Each provider gets its own isolated cache directory under
/// `$XDG_DATA_HOME/all_querier/foyer/<name>/v1/`.
pub async fn create_provider_cache(name: &str) -> anyhow::Result<ProviderCache> {
    let cache_dir = all_querier_cache_dir()
        .join("foyer")
        .join(name)
        .join(PROVIDER_CACHE_SCHEMA_VERSION);

    let device = FsDeviceBuilder::new(cache_dir)
        .with_capacity(PROVIDER_DISK_CACHE_CAPACITY_BYTES)
        .build()?;

    let engine_config = BlockEngineConfig::new(device)
        .with_flushers(1)
        .with_tombstone_log(true);

    let cache = HybridCacheBuilder::new()
        .with_name(format!("allq {name} cache"))
        .with_policy(HybridCachePolicy::WriteOnInsertion)
        .memory(PROVIDER_MEMORY_CACHE_CAPACITY)
        .storage()
        .with_recover_mode(RecoverMode::Strict)
        .with_engine_config(engine_config)
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create {name} cache: {}", e))?;

    Ok(cache)
}

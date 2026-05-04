use std::path::PathBuf;

use foyer::{BlockEngineConfig, DeviceBuilder, FsDeviceBuilder, HybridCache, HybridCacheBuilder, HybridCachePolicy, RecoverMode};
use serde_json::Value;

pub type WikidataCache = HybridCache<String, Value>;

const WIKIDATA_CACHE_SCHEMA_VERSION: &str = "v1";
const WIKIDATA_MEMORY_CACHE_CAPACITY: usize = 64 * 1024 * 1024;
const WIKIDATA_DISK_CACHE_CAPACITY_BYTES: usize = 1024 * 1024 * 1024;

pub async fn create_wikidata_cache() -> anyhow::Result<WikidataCache> {
    let cache_dir = PathBuf::from(".cache")
        .join("foyer")
        .join("wikidata")
        .join(WIKIDATA_CACHE_SCHEMA_VERSION);

    let device = FsDeviceBuilder::new(cache_dir)
        .with_capacity(WIKIDATA_DISK_CACHE_CAPACITY_BYTES)
        .build()?;

    let cache = HybridCacheBuilder::new()
        .with_name("allq wikidata cache")
        .with_policy(HybridCachePolicy::WriteOnInsertion)
        .memory(WIKIDATA_MEMORY_CACHE_CAPACITY)
        .storage()
        .with_recover_mode(RecoverMode::Quiet)
        .with_engine_config(BlockEngineConfig::new(device).with_tombstone_log(true))
        .build()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create Wikidata cache: {}", e))?;

    Ok(cache)
}

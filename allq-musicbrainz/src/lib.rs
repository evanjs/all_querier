use allq_core::{FetchMode, ProviderCache, SearchOptions, SearchProvider, SearchResult};
use async_trait::async_trait;
use musicbrainz_rs::client::MusicBrainzClient;
use musicbrainz_rs::entity::artist::Artist;
use musicbrainz_rs::entity::release_group::ReleaseGroup;
use musicbrainz_rs::entity::recording::Recording;
use musicbrainz_rs::prelude::*;
use tracing::debug;

pub const SUPPORTED_TYPES: &[&str] = &["artist", "album", "song"];

/// A `SearchProvider` backed by the MusicBrainz API.
pub struct MusicBrainzSearchProvider {
    client: MusicBrainzClient,
    cache: Option<ProviderCache>,
}

impl MusicBrainzSearchProvider {
    /// Create a new provider with the given user-agent string and no cache.
    ///
    /// The user-agent should follow MusicBrainz conventions:
    /// `ApplicationName/version ( contact-url-or-email )`
    pub fn new(user_agent: &str) -> Self {
        Self {
            client: MusicBrainzClient::new(user_agent),
            cache: None,
        }
    }

    /// Create a new provider with the given user-agent string and a foyer hybrid cache.
    pub fn new_with_cache(user_agent: &str, cache: ProviderCache) -> Self {
        Self {
            client: MusicBrainzClient::new(user_agent),
            cache: Some(cache),
        }
    }

    /// Look up a cached JSON string for `key`, respecting `fetch_mode`.
    ///
    /// Returns `Some(json)` on a cache hit (unless `ForceFetch`), `None` otherwise.
    async fn cache_get(&self, key: &str, fetch_mode: FetchMode) -> Option<String> {
        if fetch_mode == FetchMode::ForceFetch {
            return None;
        }
        let cache = self.cache.as_ref()?;
        cache.get(key).await.ok().flatten().map(|e| e.value().clone())
    }

    /// Insert `value` into the cache under `key` unless we are in `CacheOnly` mode
    /// (in which case there is nothing new to store).
    async fn cache_insert(&self, key: String, value: String, fetch_mode: FetchMode) {
        if fetch_mode == FetchMode::CacheOnly {
            return;
        }
        if let Some(cache) = &self.cache {
            cache.insert(key, value);
        }
    }
}

#[async_trait]
impl SearchProvider for MusicBrainzSearchProvider {
    fn name(&self) -> &'static str {
        "musicbrainz"
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let types_to_search: Vec<&str> = match item_type {
            Some(t) if SUPPORTED_TYPES.contains(&t) => vec![t],
            Some(t) => anyhow::bail!("unsupported item type for MusicBrainz: {t}"),
            None => SUPPORTED_TYPES.to_vec(),
        };

        let mut results = Vec::new();

        for &typ in &types_to_search {
            match typ {
                "artist" => {
                    results.extend(self.search_artists(query, options).await?);
                }
                "album" => {
                    results.extend(self.search_release_groups(query, options).await?);
                }
                "song" => {
                    results.extend(self.search_recordings(query, options).await?);
                }
                _ => {}
            }
        }

        Ok(results)
    }
}

impl MusicBrainzSearchProvider {
    async fn search_artists(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz artists");

        let cache_key = format!("musicbrainz:artist:{query}");
        let fetch_mode = options.fetch_mode;

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            let entities: Vec<Artist> = serde_json::from_str(&cached)?;
            return Ok(entities
                .into_iter()
                .map(|artist| {
                    let data = serde_json::to_value(&artist).unwrap_or_default();
                    SearchResult {
                        provider: "musicbrainz".to_string(),
                        id: artist.id,
                        label: artist.name,
                        description: Some(artist.disambiguation),
                        item_type: Some("artist".to_string()),
                        data,
                    }
                })
                .collect());
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let response = Artist::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz artist search failed: {e}"))?;

        if let Ok(json) = serde_json::to_string(&response.entities) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }

        Ok(response
            .entities
            .into_iter()
            .map(|artist| {
                let data = serde_json::to_value(&artist).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz".to_string(),
                    id: artist.id,
                    label: artist.name,
                    description: Some(artist.disambiguation),
                    item_type: Some("artist".to_string()),
                    data,
                }
            })
            .collect())
    }

    async fn search_release_groups(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz release groups");

        let cache_key = format!("musicbrainz:album:{query}");
        let fetch_mode = options.fetch_mode;

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            let entities: Vec<ReleaseGroup> = serde_json::from_str(&cached)?;
            return Ok(entities
                .into_iter()
                .map(|rg| {
                    let data = serde_json::to_value(&rg).unwrap_or_default();
                    SearchResult {
                        provider: "musicbrainz".to_string(),
                        id: rg.id,
                        label: rg.title,
                        description: Some(rg.disambiguation),
                        item_type: Some("album".to_string()),
                        data,
                    }
                })
                .collect());
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let response = ReleaseGroup::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz release group search failed: {e}"))?;

        if let Ok(json) = serde_json::to_string(&response.entities) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }

        Ok(response
            .entities
            .into_iter()
            .map(|rg| {
                let data = serde_json::to_value(&rg).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz".to_string(),
                    id: rg.id,
                    label: rg.title,
                    description: Some(rg.disambiguation),
                    item_type: Some("album".to_string()),
                    data,
                }
            })
            .collect())
    }

    async fn search_recordings(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz recordings");

        let cache_key = format!("musicbrainz:song:{query}");
        let fetch_mode = options.fetch_mode;

        if let Some(cached) = self.cache_get(&cache_key, fetch_mode).await {
            let entities: Vec<Recording> = serde_json::from_str(&cached)?;
            return Ok(entities
                .into_iter()
                .map(|rec| {
                    let data = serde_json::to_value(&rec).unwrap_or_default();
                    SearchResult {
                        provider: "musicbrainz".to_string(),
                        id: rec.id,
                        label: rec.title,
                        description: rec.disambiguation,
                        item_type: Some("song".to_string()),
                        data,
                    }
                })
                .collect());
        }

        if fetch_mode == FetchMode::CacheOnly {
            return Ok(Vec::new());
        }

        let response = Recording::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz recording search failed: {e}"))?;

        if let Ok(json) = serde_json::to_string(&response.entities) {
            self.cache_insert(cache_key, json, fetch_mode).await;
        }

        Ok(response
            .entities
            .into_iter()
            .map(|rec| {
                let data = serde_json::to_value(&rec).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz".to_string(),
                    id: rec.id,
                    label: rec.title,
                    description: rec.disambiguation,
                    item_type: Some("song".to_string()),
                    data,
                }
            })
            .collect())
    }
}

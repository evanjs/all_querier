use allq_core::{SearchOptions, SearchProvider, SearchResult};
use async_trait::async_trait;
use musicbrainz_rs::client::MusicBrainzClient;
use musicbrainz_rs::entity::artist::Artist;
use musicbrainz_rs::entity::release_group::ReleaseGroup;
use musicbrainz_rs::entity::recording::Recording;
use musicbrainz_rs::prelude::*;
use tracing::debug;

const SUPPORTED_TYPES: &[&str] = &["artist", "album", "song"];

/// A `SearchProvider` backed by the MusicBrainz API.
pub struct MusicBrainzSearchProvider {
    client: MusicBrainzClient,
}

impl MusicBrainzSearchProvider {
    /// Create a new provider with the given user-agent string.
    ///
    /// The user-agent should follow MusicBrainz conventions:
    /// `ApplicationName/version ( contact-url-or-email )`
    pub fn new(user_agent: &str) -> Self {
        Self {
            client: MusicBrainzClient::new(user_agent),
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
        _options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz artists");

        let response = Artist::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz artist search failed: {e}"))?;

        Ok(response
            .entities
            .into_iter()
            .map(|artist| {
                let data = serde_json::to_value(&artist).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz",
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
        _options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz release groups");

        let response = ReleaseGroup::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz release group search failed: {e}"))?;

        Ok(response
            .entities
            .into_iter()
            .map(|rg| {
                let data = serde_json::to_value(&rg).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz",
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
        _options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        debug!(query, "searching MusicBrainz recordings");

        let response = Recording::search(query.to_string())
            .execute_with_client_async(&self.client)
            .await
            .map_err(|e| anyhow::anyhow!("MusicBrainz recording search failed: {e}"))?;

        Ok(response
            .entities
            .into_iter()
            .map(|rec| {
                let data = serde_json::to_value(&rec).unwrap_or_default();
                SearchResult {
                    provider: "musicbrainz",
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

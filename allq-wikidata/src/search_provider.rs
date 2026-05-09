use allq_core::{SearchOptions, SearchProvider, SearchResult};
use async_trait::async_trait;

use crate::{
    CURATED_WIKIDATA_ITEM_TYPE_KEYS, SearchItemsByInstanceOfOptions, WikidataClient,
    resolve_wikidata_item_type_qid, search_items_by_instance_of_with_options,
};

/// A [`SearchProvider`] backed by Wikidata's SPARQL endpoint.
pub struct WikidataSearchProvider {
    /// Kept for future use (entity hydration, enrichment, etc.).
    #[allow(dead_code)]
    client: WikidataClient,
}

impl WikidataSearchProvider {
    pub fn new(client: WikidataClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SearchProvider for WikidataSearchProvider {
    fn name(&self) -> &'static str {
        "wikidata"
    }

    fn supported_item_types(&self) -> &[&str] {
        CURATED_WIKIDATA_ITEM_TYPE_KEYS
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let type_qid = match item_type {
            Some(t) => resolve_wikidata_item_type_qid(t)?,
            None => anyhow::bail!("wikidata search requires an item_type"),
        };

        let opts = SearchItemsByInstanceOfOptions {
            output_limit: options.limit.map(|l| l as usize),
            ..Default::default()
        };

        let results = search_items_by_instance_of_with_options(&type_qid, query, opts).await?;

        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                provider: "wikidata",
                id: r.id,
                label: r.label.clone(),
                description: r.description,
                item_type: item_type.map(|s| s.to_string()),
                data: serde_json::to_value(&r.label).unwrap_or_default(),
            })
            .collect())
    }
}

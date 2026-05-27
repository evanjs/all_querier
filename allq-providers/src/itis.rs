use async_trait::async_trait;
use tracing::debug;
use serde_json::Value;
use crate::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderLinkRoute,
    ProviderPageData,
};

const SOURCE: &str = "itis";

/// Wikidata property for associated taxon entity.
const WIKIDATA_PROPERTY_ID: &str = "P225";

pub(crate) const LINK_ALIASES: &[&str] = &[
    "itis"
];

pub static PROVIDER: ItisProvider = ItisProvider;

pub struct ItisProvider;

pub(crate) fn resolve_link_route(normalized_link: &str) -> Option<ProviderLinkRoute> {
    match normalized_link {
        "itis" => {
            Some(ProviderLinkRoute::new(&PROVIDER, WIKIDATA_PROPERTY_ID))
        }
        _ => None,
    }
}

#[async_trait]
impl ExternalIdPageProvider for ItisProvider {
    fn source(&self) -> &'static str {
        SOURCE
    }

    /// Returns the canonical itis page URL for the given page title/slug.
    fn page_url(&self, value: &str) -> String {

        // how should we handle this?
        // itis does not have distinct "pages" for each taxon, etc.
        "".to_string()
    }

    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);
        todo!("Implement itis page data fetching");

        let body = "".to_string();
        Ok(ProviderPageData { source: self.source(), url, body })
    }

    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value> {
        todo!("Implement itis page data parsing")
    }
}

use async_trait::async_trait;
use tracing::debug;
use serde_json::Value;
use crate::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderLinkRoute,
    ProviderPageData,
};

const SOURCE: &str = "pcgw";

/// Wikidata property for PCGamingWiki page ID.
const WIKIDATA_PROPERTY_ID: &str = "P6337";

pub(crate) const LINK_ALIASES: &[&str] = &[
    "pcgw",
    "pcgamingwiki",
    "pc-gaming-wiki",
];

pub static PROVIDER: PcgwProvider = PcgwProvider;

pub struct PcgwProvider;

pub(crate) fn resolve_link_route(normalized_link: &str) -> Option<ProviderLinkRoute> {
    match normalized_link {
        "pcgw" | "pcgamingwiki" | "pc-gaming-wiki" => {
            Some(ProviderLinkRoute::new(&PROVIDER, WIKIDATA_PROPERTY_ID))
        }
        _ => None,
    }
}

#[async_trait]
impl ExternalIdPageProvider for PcgwProvider {
    fn source(&self) -> &'static str {
        SOURCE
    }

    /// Returns the canonical PCGW wiki page URL for the given page title/slug.
    fn page_url(&self, value: &str) -> String {
        allq_pcgw::page_url(value)
    }

    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);
        let api_url = allq_pcgw::parse_api_url(value);

        debug!(
            source = self.source(),
            %url,
            %api_url,
            "fetching PCGW page via MediaWiki parse API",
        );

        let body = http_client.get_text(&api_url).await?;

        debug!(
            source = self.source(),
            %url,
            bytes = body.len(),
            "fetched PCGW page",
        );

        Ok(ProviderPageData { source: self.source(), url, body })
    }

    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value> {
        allq_pcgw::parse_page_response(&page_data.body)
    }
}

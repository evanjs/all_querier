use async_trait::async_trait;
use tracing::debug;
use serde_json::Value;
use std::env;
use crate::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderLinkRoute,
    ProviderPageData,
};

const SOURCE: &str = "mal";

/// Wikidata property for MyAnimeList anime ID.
const WIKIDATA_PROPERTY_ID: &str = "P4086";

pub(crate) const LINK_ALIASES: &[&str] = &[
    "mal",
    "myanimelist",
];

pub static PROVIDER: MalProvider = MalProvider;

pub struct MalProvider;

pub(crate) fn resolve_link_route(normalized_link: &str) -> Option<ProviderLinkRoute> {
    match normalized_link {
        "mal" | "myanimelist" => {
            Some(ProviderLinkRoute::new(&PROVIDER, WIKIDATA_PROPERTY_ID))
        }
        _ => None,
    }
}

#[async_trait]
impl ExternalIdPageProvider for MalProvider {
    fn source(&self) -> &'static str {
        SOURCE
    }

    fn page_url(&self, value: &str) -> String {
        allq_mal::page_url(value)
    }

    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);
        let api_url = allq_mal::api_url(value);

        debug!(
            source = self.source(),
            %url,
            %api_url,
            "fetching MyAnimeList page via API",
        );

        let client_id_str = allq_mal::get_client_id()?;

        // We bypass http_client.get_text to inject the X-MAL-CLIENT-ID header using the inner reqwest client. 
        // ProviderHttpClient doesn't expose a way to inject headers natively without modifying the common struct.
        // Or we can just use `allq_mal::MalProvider::new()` and bypass ProviderHttpClient entirely.
        
        let body = http_client.inner
            .get(&api_url)
            .header("X-MAL-CLIENT-ID", client_id_str)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        debug!(
            source = self.source(),
            %url,
            bytes = body.len(),
            "fetched MAL page",
        );

        Ok(ProviderPageData { source: self.source(), url, body })
    }

    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value> {
        allq_mal::parse_page_response(&page_data.body)
    }
}

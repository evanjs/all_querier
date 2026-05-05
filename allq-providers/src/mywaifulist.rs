use async_trait::async_trait;
use tracing::debug;
use serde_json::Value;
use anyhow::Context;
use crate::{
    ExternalIdPageProvider,
    ProviderHttpClient,
    ProviderLinkRoute,
    ProviderPageData,
};

const SOURCE: &str = "mywaifulist";
const WIKIDATA_PROPERTY_ID: &str = "P13031";

pub(crate) const LINK_ALIASES: &[&str] = &[
    "waifu",
    "mywaifulist",
    "my-waifu-list",
];

pub static PROVIDER: MyWaifuListProvider = MyWaifuListProvider;

pub struct MyWaifuListProvider;

pub(crate) fn resolve_link_route(normalized_link: &str) -> Option<ProviderLinkRoute> {
    match normalized_link {
        "waifu" | "mywaifulist" | "my-waifu-list" => {
            Some(ProviderLinkRoute::new(&PROVIDER, WIKIDATA_PROPERTY_ID))
        }
        _ => None,
    }
}

#[async_trait]
impl ExternalIdPageProvider for MyWaifuListProvider {
    fn source(&self) -> &'static str {
        SOURCE
    }

    fn page_url(&self, value: &str) -> String {
        format!("https://mywaifulist.moe/waifu/{value}")
    }

    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);

        debug!(
            source = self.source(),
            %url,
            "fetching provider page",
        );

        let body = http_client.get_text(&url).await?;

        debug!(
            source = self.source(),
            %url,
            bytes = body.len(),
            "fetched provider page",
        );

        Ok(ProviderPageData { source: self.source(), url, body })
    }

    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value> {
        let needle = "data-page=\"app\" type=\"application/json\">";
        let start = page_data.body.find(needle).map(|i| i + needle.len()).context("missing inertia script")?;
        let end = page_data.body[start..].find("</script>").context("missing closing script")?;
        let root: Value = serde_json::from_str(&page_data.body[start..start+end])?;
        root.pointer("/props/waifu").cloned().context("waifu prop missing")
    }
}
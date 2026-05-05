use async_trait::async_trait;
use tracing::debug;
use serde_json::Value;
use anyhow::Context;
use crate::{ExternalIdPageProvider, ProviderPageData};

pub struct MyWaifuListProvider;

#[async_trait]
impl ExternalIdPageProvider for MyWaifuListProvider {
    fn source(&self) -> &'static str { "mywaifulist" }
    fn page_url(&self, value: &str) -> String { format!("https://mywaifulist.moe/waifu/{value}") }
    async fn fetch_page_data(&self, value: &str) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);

        debug!(
            source = self.source(),
            %url,
            "fetching provider page",
        );

        let body = reqwest::Client::builder().user_agent("allq/0.1.0").build()?
            .get(&url).send().await?.text().await?;

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
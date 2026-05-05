use anyhow::Context;
use async_trait::async_trait;
use serde_json::Value;

pub struct ProviderPageData {
    pub source: &'static str,
    pub url: String,
    pub body: String,
}

#[async_trait]
pub trait ExternalIdPageProvider {
    fn source(&self) -> &'static str;
    fn page_url(&self, value: &str) -> String;
    async fn fetch_page_data(&self, value: &str) -> anyhow::Result<ProviderPageData>;
    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value>;
}

pub struct MyWaifuListProvider;

#[async_trait]
impl ExternalIdPageProvider for MyWaifuListProvider {
    fn source(&self) -> &'static str { "mywaifulist" }
    fn page_url(&self, value: &str) -> String { format!("https://mywaifulist.moe/waifu/{value}") }
    async fn fetch_page_data(&self, value: &str) -> anyhow::Result<ProviderPageData> {
        let url = self.page_url(value);
        let body = reqwest::Client::builder().user_agent("allq/0.1.0").build()?
            .get(&url).send().await?.text().await?;
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
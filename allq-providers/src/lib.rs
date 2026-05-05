use anyhow::Context;
use async_trait::async_trait;
use serde_json::Value;
pub mod mywaifulist;

static APP_USER_AGENT: &str = concat!(
env!("CARGO_PKG_NAME"),
"/",
env!("CARGO_PKG_VERSION"),
);


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

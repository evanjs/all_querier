use anyhow::Context;
use async_trait::async_trait;
use serde_json::Value;

pub mod mywaifulist;

pub const APP_USER_AGENT: &str = concat!(
env!("CARGO_PKG_NAME"),
"/",
env!("CARGO_PKG_VERSION"),
);

#[derive(Debug, Clone)]
pub struct ProviderHttpClient {
    inner: reqwest::Client,
}

impl ProviderHttpClient {
    pub fn new() -> anyhow::Result<Self> {
        Self::with_user_agent(APP_USER_AGENT)
    }

    pub fn with_user_agent(user_agent: &str) -> anyhow::Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .context("failed to build provider HTTP client")?;

        Ok(Self { inner })
    }

    pub async fn get_text(&self, url: &str) -> anyhow::Result<String> {
        self.inner
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to fetch provider page: {url}"))?
            .error_for_status()
            .with_context(|| format!("provider page returned an error status: {url}"))?
            .text()
            .await
            .with_context(|| format!("failed to read provider page body: {url}"))
    }
}

pub struct ProviderPageData {
    pub source: &'static str,
    pub url: String,
    pub body: String,
}

#[async_trait]
pub trait ExternalIdPageProvider {
    fn source(&self) -> &'static str;
    fn page_url(&self, value: &str) -> String;
    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData>;
    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value>;
}
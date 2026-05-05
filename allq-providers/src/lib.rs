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
pub trait ExternalIdPageProvider: Sync {
    fn source(&self) -> &'static str;
    fn page_url(&self, value: &str) -> String;
    async fn fetch_page_data(
        &self,
        http_client: &ProviderHttpClient,
        value: &str,
    ) -> anyhow::Result<ProviderPageData>;
    fn parse_page_data(&self, page_data: &ProviderPageData) -> anyhow::Result<Value>;
}

#[derive(Clone, Copy)]
pub struct ProviderLinkRoute {
    provider: &'static dyn ExternalIdPageProvider,
    property_id: &'static str,
}

impl ProviderLinkRoute {
    pub(crate) fn new(
        provider: &'static dyn ExternalIdPageProvider,
        property_id: &'static str,
    ) -> Self {
        Self {
            provider,
            property_id,
        }
    }

    pub fn source(self) -> &'static str {
        self.provider.source()
    }

    pub fn property_id(self) -> &'static str {
        self.property_id
    }

    pub fn provider(self) -> &'static dyn ExternalIdPageProvider {
        self.provider
    }
}

pub fn resolve_provider_link(link: &str) -> anyhow::Result<ProviderLinkRoute> {
    let normalized_link = normalize_link_key(link);

    if let Some(route) = mywaifulist::resolve_link_route(&normalized_link) {
        return Ok(route);
    }

    let supported = supported_provider_link_aliases().join(", ");
    anyhow::bail!("unsupported link type {link:?}; supported link types: {supported}")
}

fn supported_provider_link_aliases() -> Vec<&'static str> {
    let mut aliases = Vec::new();
    aliases.extend_from_slice(mywaifulist::LINK_ALIASES);
    aliases
}

fn normalize_link_key(link: &str) -> String {
    link.trim().to_ascii_lowercase()
}
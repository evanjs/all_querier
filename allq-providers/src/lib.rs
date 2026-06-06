use anyhow::Context;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

pub mod mywaifulist;
pub mod pcgw;
pub mod mal;
pub mod itis;

pub fn app_user_agent() -> String {
    let authors = env!("CARGO_PKG_AUTHORS");
    let author = authors.split(':').next().unwrap_or(authors);
    let email = author
        .split_once('<')
        .and_then(|(_, rest)| rest.split_once('>'))
        .map(|(email, _)| email.trim())
        .unwrap_or(author.trim());
    format!(
        "{}/{} ({}) reqwest/{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        email,
        allq_core::REQWEST_VERSION,
    )
}

#[derive(Debug, Clone)]
pub struct ProviderHttpClient {
    inner: reqwest::Client,
}

impl ProviderHttpClient {
    pub fn new() -> anyhow::Result<Self> {
        Self::with_user_agent(&app_user_agent())
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

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SupportedProviderLink {
    pub primary_alias: &'static str,
    pub aliases: &'static [&'static str],
    pub source: &'static str,
    pub property_id: &'static str,
    pub supported_item_types: &'static [&'static str],
    pub description: &'static str,
}

macro_rules! supported_provider_links {
    (
        $(
            {
                primary_alias: $primary_alias:literal,
                aliases: $aliases:path,
                source: $source:literal,
                property_id: $property_id:literal,
                supported_item_types: [$($supported_item_type:literal),+ $(,)?],
                description: $description:literal $(,)?
            }
        ),+ $(,)?
    ) => {
        pub const SUPPORTED_PROVIDER_LINKS: &[SupportedProviderLink] = &[
            $(
                SupportedProviderLink {
                    primary_alias: $primary_alias,
                    aliases: $aliases,
                    source: $source,
                    property_id: $property_id,
                    supported_item_types: &[
                        $(
                            $supported_item_type,
                        )+
                    ],
                    description: $description,
                },
            )+
        ];

        pub const SUPPORTED_PROVIDER_LINK_PRIMARY_ALIASES: &[&str] = &[
            $(
                $primary_alias,
            )+
        ];
    };
}

supported_provider_links! {
    {
        primary_alias: "waifu",
        aliases: mywaifulist::LINK_ALIASES,
        source: "mywaifulist",
        property_id: "P13031",
        supported_item_types: ["character"],
        description: "Fetch MyWaifuList character page data",
    },
    {
        primary_alias: "pcgw",
        aliases: pcgw::LINK_ALIASES,
        source: "pcgw",
        property_id: "P6337",
        supported_item_types: ["video-game"],
        description: "Fetch PCGamingWiki game page data",
    },
    {
        primary_alias: "mal",
        aliases: mal::LINK_ALIASES,
        source: "mal",
        property_id: "P4086",
        supported_item_types: ["anime", "manga"],
        description: "Fetch MyAnimeList anime page data",
    },
    {
        primary_alias: "taxon",
        aliases: itis::LINK_ALIASES,
        source: "itis",
        property_id: "P225",
        supported_item_types: [ "taxon" ],
        description: "Fetch taxonomy data from itis.gov"
    }
}

pub fn supported_provider_links() -> &'static [SupportedProviderLink] {
    SUPPORTED_PROVIDER_LINKS
}

pub fn supported_provider_links_for_type(
    item_type: Option<&str>,
) -> impl Iterator<Item = &'static SupportedProviderLink> {
    let normalized_item_type = item_type.map(normalize_link_key);

    supported_provider_links()
        .iter()
        .filter(move |link| {
            let Some(item_type) = normalized_item_type.as_deref() else {
                return true;
            };

            link.supported_item_types
                .iter()
                .any(|supported_item_type| supported_item_type.eq_ignore_ascii_case(item_type))
        })
}

pub fn resolve_provider_link(link: &str) -> anyhow::Result<ProviderLinkRoute> {
    let normalized_link = normalize_link_key(link);

    if let Some(route) = mywaifulist::resolve_link_route(&normalized_link) {
        return Ok(route);
    }

    if let Some(route) = pcgw::resolve_link_route(&normalized_link) {
        return Ok(route);
    }

    if let Some(route) = mal::resolve_link_route(&normalized_link) {
        return Ok(route);
    }

    let supported = supported_provider_link_aliases().join(", ");
    anyhow::bail!("unsupported link type {link:?}; supported link types: {supported}")
}

pub fn supported_provider_link_aliases() -> Vec<&'static str> {
    supported_provider_links()
        .iter()
        .flat_map(|link| link.aliases.iter().copied())
        .collect()
}

fn normalize_link_key(link: &str) -> String {
    link.trim().to_ascii_lowercase()
}
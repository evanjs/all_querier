use allq_core::{SearchOptions, SearchProvider, SearchResult};
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

pub const SUPPORTED_TYPES: &[&str] = &["default_type"];

/// A `SearchProvider` backed by the {{provider_struct}} API.
pub struct {{ provider_struct }} {
    client: reqwest::Client,
    // Add additional state here, e.g., rate-limiters, specific API wrappers, caching
}

impl {{ provider_struct }} {
    pub fn new() -> Result<Self> {
        // Initialize any configuration or credentials here
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;

        Ok(Self { client })
    }
}

#[async_trait]
impl SearchProvider for {{ provider_struct }} {
    fn name(&self) -> &'static str {
        "{{provider_name }}"
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let itype = item_type.unwrap_or("default_type");
        let limit = options.limit.unwrap_or(10);

        // Consider using tracing so the module can be more easily debugged
        debug!(
            ?query,
            itype = %itype,
            limit = limit,
            "Searching {{ provider_name}}"
        );

        // TODO: Map query options to specific {{provider_name}} API requests and parameters
        // let mut results = Vec::new();
        // let response = self.client.get("https://api.example.com/search")
        //      .query(&[("q", query), ("limit", &limit.to_string())])
        //      .send()
        //      .await?;

        // TODO: Parse the external response into standard `SearchResult`s
        todo!("Implement backend requesting and parsing for {{ provider_struct }}")
    }
}
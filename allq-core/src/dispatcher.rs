use crate::{GameSearchOptions, SearchOptions, SearchProvider, SearchResult};

/// Holds multiple [`SearchProvider`] implementations and fans out queries.
pub struct SearchDispatcher {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchDispatcher {
    /// Create a new dispatcher with no providers.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a provider.
    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    /// Build a dispatcher from an iterator of providers.
    pub fn from_providers(providers: impl IntoIterator<Item = Box<dyn SearchProvider>>) -> Self {
        Self {
            providers: providers.into_iter().collect(),
        }
    }

    /// Search all registered providers that support the given item type.
    ///
    /// Results are collected sequentially for now; parallel fan-out can be
    /// added later via `tokio::join!` or `futures::join_all`.
    pub async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        for provider in &self.providers {
            if item_type.map_or(true, |t| provider.supported_item_types().contains(&t)) {
                tracing::debug!("Searching provider: {}", provider.name());
                let provider_results = provider.search(query, item_type, options).await?;
                tracing::debug!("Provider {} returned {} results", provider.name(), provider_results.len());
                results.extend(provider_results);
            }
        }
        Ok(results)
    }

    /// Search all registered providers that support video games.
    ///
    /// Results are collected sequentially for now; parallel fan-out can be
    /// added later via `tokio::join!` or `futures::join_all`.
    pub async fn search_games(
        &self,
        query: &str,
        options: &GameSearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let search_options = SearchOptions::from(options.clone());

        for provider in &self.providers {
            if !provider.supported_item_types().contains(&"video-game") {
                continue;
            }

            tracing::debug!("Searching provider: {}", provider.name());
            let provider_results = provider
                .search(query, Some("video-game"), &search_options)
                .await?;
            tracing::debug!(
                "Provider {} returned {} results",
                provider.name(),
                provider_results.len()
            );
            results.extend(provider_results);
        }

        Ok(results)
    }

    /// Return the names of all registered providers.
    pub fn provider_names(&self) -> Vec<&'static str> {
        self.providers.iter().map(|p| p.name()).collect()
    }
}

impl Default for SearchDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

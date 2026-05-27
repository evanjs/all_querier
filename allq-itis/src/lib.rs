mod queries;

use allq_core::{SearchOptions, SearchProvider, SearchResult};
use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Connection, SqlitePool};
use sqlx::sqlite::SqlitePoolOptions;
use tracing::debug;
use crate::queries::search_taxon_by_vernacular;

pub const SUPPORTED_TYPES: &[&str] = &["taxon"];
const WIKIDATA_PROPERTY_ID: &str = "P225";
pub(crate) const LINK_ALIASES: &[&str] = &[
    "itis",
];

/// A `SearchProvider` backed by the ItisProvider API.
pub struct ItisProvider {
    client: reqwest::Client,
    // Add additional state here, e.g., rate-limiters, specific API wrappers, caching
}

impl ItisProvider {
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
impl SearchProvider for ItisProvider {
    fn name(&self) -> &'static str {
        "itis"
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
            "Searching itis"
        );

        // let connection = sqlx::SqliteConnection::connect(
        //     "sqlite:./scratch/itisSqlite050426/ITIS.sqlite"
        // ).await?;

        let itis_path = allq_core::all_querier_data_dir()
            .unwrap_or(std::env::current_dir()?)
            .join("data")
            .join("ITIS.sqlite");
        let itis_path_connection_string = format!(
            "sqlite:{}", itis_path.as_path().display()
        );
        debug!(
            ?itis_path,
            ?itis_path_connection_string,
        );
        let pool: SqlitePool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect(&itis_path_connection_string).await?;

        // for "standard" search without parameters
        // let's assume the user is searching using the "common name" (itis: vernacular)
        // simply return the related taxonomy in this case
        // TODO: Map query options to specific itis API requests and parameters
        let mut results = vec![];
        let result = search_taxon_by_vernacular(pool, query).await?;
        let data = serde_json::to_value(&result)?;
        results.push(SearchResult {
            data,
            provider: "itis".to_string(),
            id: result.tsn.to_string(),
            label: result.vernacular_name.to_string(),
            description: Some(result.complete_name.to_string()),
            item_type: Some("taxon".to_string())
        });

        // let response = self.client.get("https://api.example.com/search")
        //      .query(&[("q", query), ("limit", &limit.to_string())])
        //      .send()
        //      .await?;

        Ok(results)

        // TODO: Parse the external response into standard `SearchResult`s
        // todo!("Implement backend requesting and parsing for ItisProvider")
    }

    fn supported_item_types(&self) -> &[&str] {
        SUPPORTED_TYPES
    }
}
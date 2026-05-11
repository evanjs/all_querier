use allq_core::{SearchProvider, SearchOptions, SearchResult, all_querier_data_dir};
use anyhow::Result;
use async_trait::async_trait;
use myanimelist::{MalClient, ClientId};
use serde::Deserialize;
use std::env;
use std::fs;

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga"];

pub fn page_url(id: &str) -> String {
    format!("https://myanimelist.net/anime/{id}")
}

pub fn api_url(id: &str) -> String {
    format!("https://api.myanimelist.net/v2/anime/{id}?fields=id,title,main_picture,alternative_titles,start_date,end_date,synopsis,mean,rank,popularity,num_list_users,num_scoring_users,nsfw,created_at,updated_at,media_type,status,genres,my_list_status,num_episodes,start_season,broadcast,source,average_episode_duration,rating,pictures,background,related_anime,related_manga,recommendations,studios,statistics")
}

pub fn parse_page_response(body: &str) -> Result<serde_json::Value> {
    let root: serde_json::Value = serde_json::from_str(body)?;
    Ok(root)
}

#[derive(Deserialize)]
struct MalConfig {
    mal_client_id: String,
}

pub fn get_client_id() -> Result<String> {
    if let Ok(id) = env::var("MAL_CLIENT_ID") {
        return Ok(id);
    }
    
    let path = all_querier_data_dir().join("credentials.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<MalConfig>(&content) {
                return Ok(config.mal_client_id);
            }
        }
    }
    
    anyhow::bail!("Please set MAL_CLIENT_ID in environment or in {:?}", path)
}

pub struct MalProvider {
    client: MalClient,
}

impl MalProvider {
    pub fn new() -> Result<Self> {
        let client_id_str = get_client_id()?;
        
        let client_id = ClientId::new(client_id_str);
        
        // Build the client without real user tokens, only relying on the client id
        // The myanimelist crate builder requires auth_tokens to be provided, 
        // so we just pass default dummy tokens. The public endpoints we use 
        // will use the client_id header instead of the bearer auth.
        let client = MalClient::builder()
            .client_id(client_id)
            .auth_tokens(myanimelist::auth::AuthTokens::default())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build MyAnimeList client: {}", e))?;
            
        Ok(Self { client })
    }
}

#[async_trait]
impl SearchProvider for MalProvider {
    fn name(&self) -> &'static str {
        "myanimelist"
    }

    fn supported_item_types(&self) -> &[&str] {
        &["anime", "manga"]
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        _options: &SearchOptions, // could use limit etc.
    ) -> Result<Vec<SearchResult>> {
        let itype = item_type.unwrap_or("anime");
        let mut results = Vec::new();
        
        if itype == "anime" || itype == "all" {
            let anime_results = self.client.anime().get().list().q(query).limit(10).send().await?;
            for edge in anime_results.data {
                let node = edge.node;
                results.push(SearchResult {
                    label: node.title.clone(),
                    id: node.id.to_string(),
                    item_type: Some("anime".to_string()),
                    provider: "myanimelist".to_string(),
                    description: None,
                    data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                });
            }
        }
        
        if itype == "manga" || itype == "all" {
            let manga_results = self.client.manga().get().list().q(query).limit(10).send().await?;
            for edge in manga_results.data {
                let node = edge.node;
                results.push(SearchResult {
                    label: node.title.clone(),
                    id: node.id.to_string(),
                    item_type: Some("manga".to_string()),
                    provider: "myanimelist".to_string(),
                    description: None,
                    data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                });
            }
        }
        
        Ok(results)
    }
}

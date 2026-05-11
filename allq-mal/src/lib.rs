use allq_core::{SearchProvider, SearchOptions, SearchResult, all_querier_data_dir};
use anyhow::Result;
use async_trait::async_trait;
use myanimelist::{MalClient, ClientId};
use serde::Deserialize;
use std::env;
use std::fs;

pub const SUPPORTED_TYPES: &[&str] = &["anime", "manga", "animelist", "mangalist"];

/// All valid MAL media sub-type strings (anime + manga variants).
pub const MAL_MEDIA_TYPES: &[&str] = &[
    "tv", "ova", "movie", "special", "ona", "music",
    "manga", "novel", "one_shot", "doujinshi", "manhwa", "manhua", "oel", "pv"
];

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
    access_token: Option<String>,
    refresh_token: Option<String>,
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

pub fn get_config() -> Result<MalConfig> {
    if let Ok(id) = env::var("MAL_CLIENT_ID") {
        return Ok(MalConfig {
            mal_client_id: id,
            access_token: None,
            refresh_token: None,
        });
    }

    let path = all_querier_data_dir().join("credentials.json");
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let config: MalConfig = serde_json::from_str(&content)?;
        return Ok(config);
    }

    anyhow::bail!("Please set MAL_CLIENT_ID in environment or in {:?}", path)
}

pub struct MalProvider {
    client: MalClient,
}

impl MalProvider {
    pub fn new() -> Result<Self> {
        let config = get_config()?;
        let client_id = ClientId::new(config.mal_client_id);

        // TODO: implement full auth
        // let mut auth_tokens = myanimelist::auth::AuthTokens::default();
        // if let (Some(access), Some(refresh)) = (config.access_token, config.refresh_token) {
        //     auth_tokens = myanimelist::auth::AuthTokens { access_token: access, refresh_token: refresh };
        // }
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
        &["anime", "manga", "animelist", "mangalist"]
    }

    async fn search(
        &self,
        query: &str,
        item_type: Option<&str>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let itype = item_type.unwrap_or("anime");
        let limit = options.limit.unwrap_or(10);
        // Normalize list types to base types for search
        let normalized_itype = match itype {
            "animelist" => "anime",
            "mangalist" => "manga",
            _ => itype,
        };

        // When a media_type filter is active we need to over-fetch from the API
        // so that we still return `limit` results after the post-filter step.
        // MAL catalog search caps at 100 per page; user list endpoints support up to 1000.
        let api_limit: u32 = if options.media_type.is_some() {
            100
        } else {
            limit.min(100)
        };
        let mut results = Vec::new();

        const ANIME_FIELDS: &[&str] = &[
            "id", "title", "main_picture", "alternative_titles", "start_date", "end_date",
            "synopsis", "mean", "rank", "popularity", "num_list_users", "num_scoring_users",
            "nsfw", "media_type", "status", "genres", "num_episodes", "start_season",
            "broadcast", "source", "average_episode_duration", "rating", "studios",
        ];

        const MANGA_FIELDS: &[&str] = &[
            "id", "title", "main_picture", "alternative_titles", "start_date", "end_date",
            "synopsis", "mean", "rank", "popularity", "num_list_users", "num_scoring_users",
            "nsfw", "media_type", "status", "genres", "num_volumes", "num_chapters", "authors",
        ];

        if (normalized_itype == "anime" || normalized_itype == "all") && itype != "animelist" {
            let anime_results = self.client.anime().get().list()
                .q(query)
                .limit(api_limit)
                .fields(ANIME_FIELDS)
                .nsfw(options.nsfw)
                .send().await?;
            for edge in anime_results.data {
                let node = edge.node;
                results.push(SearchResult {
                    label: node.title.clone(),
                    id: node.id.to_string(),
                    item_type: Some(itype.to_string()),
                    provider: "myanimelist".to_string(),
                    description: node.synopsis.clone(),
                    data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                });
            }
        }

        if itype == "animelist" {
            let username = options
                .mal_username
                .as_deref()
                .unwrap_or("me");

            const PAGE_SIZE: u16 = 1000;
            let mut offset: u64 = 0;
            loop {
                let page = self.client.user_animelist()
                    .get()
                    .user_name(myanimelist::objects::Username::User(username.into()))
                    .limit(PAGE_SIZE)
                    .offset(offset)
                    .nsfw(options.nsfw)
                    .send().await?;
                let has_next = page.paging.as_ref().and_then(|p| p.next.as_ref()).is_some();
                for edge in page.data {
                    let node = edge.node;
                    if !query.is_empty() && !node.title.to_lowercase().contains(&query.to_lowercase()) {
                        continue;
                    }
                    results.push(SearchResult {
                        label: node.title.clone(),
                        id: node.id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "myanimelist".to_string(),
                        description: None,
                        data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                    });
                }
                if !has_next {
                    break;
                }
                offset += PAGE_SIZE as u64;
            }
        }

        if (normalized_itype == "manga" || normalized_itype == "all") && itype != "mangalist" {
            let manga_results = self.client.manga().get().list()
                .q(query)
                .limit(api_limit as u16)
                .fields(MANGA_FIELDS)
                .nsfw(options.nsfw)
                .send().await?;
            for edge in manga_results.data {
                let node = edge.node;
                results.push(SearchResult {
                    label: node.title.clone(),
                    id: node.id.to_string(),
                    item_type: Some(itype.to_string()),
                    provider: "myanimelist".to_string(),
                    description: node.synopsis.clone(),
                    data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                });
            }
        }

        if itype == "mangalist" {
            let username = options
                .mal_username
                .as_deref()
                .unwrap_or("me");

            const PAGE_SIZE: u16 = 1000;
            let mut offset: u64 = 0;
            loop {
                let page = self.client.user_mangalist()
                    .get()
                    .user_name(myanimelist::objects::Username::User(username.into()))
                    .limit(PAGE_SIZE)
                    .offset(offset)
                    .nsfw(options.nsfw)
                    .send().await?;
                let has_next = page.paging.as_ref().and_then(|p| p.next.as_ref()).is_some();
                for edge in page.data {
                    let node = edge.node;
                    if !query.is_empty() && !node.title.to_lowercase().contains(&query.to_lowercase()) {
                        continue;
                    }
                    results.push(SearchResult {
                        label: node.title.clone(),
                        id: node.id.to_string(),
                        item_type: Some(itype.to_string()),
                        provider: "myanimelist".to_string(),
                        description: None,
                        data: serde_json::to_value(&node).unwrap_or(serde_json::Value::Null),
                    });
                }
                if !has_next {
                    break;
                }
                offset += PAGE_SIZE as u64;
            }
        }

        if let Some(ref mt) = options.media_type {
            results.retain(|r| {
                r.data
                    .get("media_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case(mt))
                    .unwrap_or(false)
            });
        }

        // Apply the user-requested limit after filtering so the final result
        // count is correct regardless of how many items were filtered out.
        results.truncate(limit as usize);

        Ok(results)
    }
}

use async_trait::async_trait;

use reqwest::Client;
use serde_json::{Value, json};

use mutsukitube_core::{ProviderError, Video, VideoProvider};

use crate::{
    models::{collect_videos, extract_continuation_token},
    search_page::SearchPage,
};

const SEARCH_ENDPOINT: &str = "https://www.youtube.com/youtubei/v1/search?prettyPrint=false";

// Versi bootstrap. Bisa diganti lewat environment variable.
const DEFAULT_WEB_VERSION: &str = "2.20261002.01.00";

pub struct NativeYoutubeProvider {
    client: Client,
    client_version: String,
}

impl NativeYoutubeProvider {
    pub fn new() -> Self {
        let client_version = std::env::var("MUTSUKITUBE_YT_CLIENT_VERSION")
            .unwrap_or_else(|_| DEFAULT_WEB_VERSION.to_string());

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/130.0.0.0 Safari/537.36",
            )
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            client_version,
        }
    }

    async fn search_native(&self, query: &str) -> Result<Vec<Video>, ProviderError> {
        let page = self.search_page(query, None).await?;

        if page.videos.is_empty() {
            return Err(ProviderError::Parse("No video results found".into()));
        }

        Ok(page.videos)
    }

    pub async fn search_page(
        &self,
        query: &str,
        continuation: Option<&str>,
    ) -> Result<SearchPage, ProviderError> {
        if query.trim().is_empty() {
            return Ok(SearchPage {
                videos: Vec::new(),
                next_page_token: None,
            });
        }

        let mut body = json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": self.client_version,
                    "hl": "en",
                    "gl": "US"
                }
            }
        });

        if let Some(token) = continuation {
            body["continuation"] = json!(token);
        } else {
            body["query"] = json!(query);
            body["params"] = json!("EgIQAQ==");
        }

        let response = self
            .client
            .post(SEARCH_ENDPOINT)
            .header("X-YouTube-Client-Name", "1")
            .header("X-YouTube-Client-Version", &self.client_version)
            .header("Origin", "https://www.youtube.com")
            .header("Referer", "https://www.youtube.com/")
            .json(&body)
            .send()
            .await
            .map_err(|error| ProviderError::Network(error.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            return Err(ProviderError::Other(format!("InnerTube HTTP {status}")));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|error| ProviderError::Parse(error.to_string()))?;

        let mut videos = Vec::new();

        collect_videos(&data, &mut videos);

        let next_page_token = extract_continuation_token(&data);

        Ok(SearchPage {
            videos,
            next_page_token,
        })
    }
}

impl Default for NativeYoutubeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for NativeYoutubeProvider {
    async fn search(&self, query: &str) -> Result<Vec<Video>, ProviderError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.search_native(query).await
    }

    async fn video(&self, _id: &str) -> Result<Video, ProviderError> {
        Err(ProviderError::Other(
            "Native video metadata is not implemented yet".into(),
        ))
    }
}

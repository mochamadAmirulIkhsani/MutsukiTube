use async_trait::async_trait;

use crate::Video;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("network error: {0}")]
    Network(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("video not found")]
    NotFound,

    #[error("provider error: {0}")]
    Other(String),
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<Video>, ProviderError>;

    async fn video(&self, id: &str) -> Result<Video, ProviderError>;
}

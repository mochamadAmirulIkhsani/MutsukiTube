use async_trait::async_trait;

use mutsukitube_core::{ProviderError, Video, VideoProvider};

use serde::Deserialize;
use tokio::process::Command;

pub struct YtDlpProvider;

impl YtDlpProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for YtDlpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct YtDlpVideo {
    id: String,
    title: String,

    #[serde(default)]
    channel: Option<String>,

    #[serde(default)]
    uploader: Option<String>,

    #[serde(default)]
    thumbnail: Option<String>,

    duration: Option<f64>,
}

impl From<YtDlpVideo> for Video {
    fn from(value: YtDlpVideo) -> Self {
        Video {
            id: value.id,

            title: value.title,

            channel: value
                .channel
                .or(value.uploader)
                .unwrap_or_else(|| "Unknown channel".to_string()),

            thumbnail: value.thumbnail.unwrap_or_default(),

            duration: value.duration.map(|duration| duration as u64),
        }
    }
}

#[async_trait]
impl VideoProvider for YtDlpProvider {
    async fn search(&self, query: &str) -> Result<Vec<Video>, ProviderError> {
        let search_query = format!("ytsearch12:{query}");

        let output = Command::new("yt-dlp")
            .args([
                &search_query,
                "--dump-json",
                "--skip-download",
                "--no-warnings",
            ])
            .output()
            .await
            .map_err(|error| ProviderError::Other(error.to_string()))?;

        if !output.status.success() {
            return Err(ProviderError::Other(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        let videos = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<YtDlpVideo>(line).ok())
            .map(Video::from)
            .collect();

        Ok(videos)
    }

    async fn video(&self, id: &str) -> Result<Video, ProviderError> {
        let url = format!("https://www.youtube.com/watch?v={id}");

        let output = Command::new("yt-dlp")
            .args([&url, "--dump-json", "--skip-download", "--no-warnings"])
            .output()
            .await
            .map_err(|error| ProviderError::Other(error.to_string()))?;

        if !output.status.success() {
            return Err(ProviderError::Other(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let video = serde_json::from_slice::<YtDlpVideo>(&output.stdout)
            .map_err(|error| ProviderError::Parse(error.to_string()))?;

        Ok(video.into())
    }
}

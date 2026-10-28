use async_trait::async_trait;

use mutsukitube_core::{ProviderError, Video, VideoProvider};

use crate::{native::NativeYoutubeProvider, ytdlp::YtDlpProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderMode {
    Auto,
    Native,
    YtDlp,
}

impl ProviderMode {
    pub fn from_env() -> Self {
        let value = std::env::var("MUTSUKITUBE_PROVIDER").unwrap_or_else(|_| "auto".into());

        match value.to_lowercase().as_str() {
            "native" => Self::Native,
            "ytdlp" => Self::YtDlp,
            _ => Self::Auto,
        }
    }
}

pub struct YoutubeProvider {
    native: NativeYoutubeProvider,
    fallback: YtDlpProvider,
    mode: ProviderMode,
}

impl YoutubeProvider {
    pub fn new() -> Self {
        Self::with_mode(ProviderMode::from_env())
    }

    pub fn with_mode(mode: ProviderMode) -> Self {
        Self {
            native: NativeYoutubeProvider::new(),
            fallback: YtDlpProvider::new(),
            mode,
        }
    }

    pub fn native(&self) -> &NativeYoutubeProvider {
        &self.native
    }

    pub async fn trending(
        &self,
        category: Option<&str>,
        country: Option<&str>,
    ) -> Result<Vec<Video>, ProviderError> {
        match self.mode {
            ProviderMode::Native => self.native.trending(category, country).await,
            ProviderMode::YtDlp => self.fallback.trending(category, country).await,
            ProviderMode::Auto => match self.native.trending(category, country).await {
                Ok(videos) if !videos.is_empty() => {
                    println!("[MutsukiTube] Native trending: {} videos", videos.len());
                    Ok(videos)
                }
                Ok(_) | Err(_) => {
                    eprintln!("[MutsukiTube] Native trending empty or failed, trying yt-dlp fallback");
                    self.fallback.trending(category, country).await
                }
            },
        }
    }

    pub async fn channel_avatar(&self, channel_id: &str) -> Result<Option<String>, ProviderError> {
        self.native.channel_avatar(channel_id).await
    }
}

impl Default for YoutubeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for YoutubeProvider {
    async fn search(&self, query: &str) -> Result<Vec<Video>, ProviderError> {
        match self.mode {
            ProviderMode::Native => self.native.search(query).await,

            ProviderMode::YtDlp => self.fallback.search(query).await,

            ProviderMode::Auto => match self.native.search(query).await {
                Ok(videos) => {
                    println!("[MutsukiTube] Native: {} results", videos.len());

                    Ok(videos)
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Native failed: {error}");

                    eprintln!("[MutsukiTube] Using yt-dlp fallback");

                    self.fallback.search(query).await
                }
            },
        }
    }

    async fn video(&self, id: &str) -> Result<Video, ProviderError> {
        self.fallback.video(id).await
    }
}

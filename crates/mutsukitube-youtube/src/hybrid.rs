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

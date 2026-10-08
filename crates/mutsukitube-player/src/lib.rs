use std::process::{Child, Command};

#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("failed to launch mpv: {0}")]
    Launch(#[from] std::io::Error),
}

pub struct ExternalMpvPlayer;

impl ExternalMpvPlayer {
    pub fn play_youtube(video_id: &str) -> Result<Child, PlayerError> {
        let url = format!("https://www.youtube.com/watch?v={video_id}");

        let child = Command::new("mpv")
            .arg(url)
            .arg("--hwdec=auto-safe")
            .arg("--force-window=yes")
            .spawn()?;

        Ok(child)
    }
}

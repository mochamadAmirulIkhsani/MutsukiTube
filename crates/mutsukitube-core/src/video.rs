use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub thumbnail: String,
    pub duration: Option<u64>,
}

impl Video {
    pub fn youtube_url(&self) -> String {
        format!("https://www.youtube.com/watch?v={}", self.id)
    }
}

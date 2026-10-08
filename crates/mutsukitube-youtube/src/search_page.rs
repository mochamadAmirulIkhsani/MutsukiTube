use mutsukitube_core::Video;

#[derive(Debug, Clone)]
pub struct SearchPage {
    pub videos: Vec<Video>,
    pub next_page_token: Option<String>,
}

impl SearchPage {
    pub fn has_next_page(&self) -> bool {
        self.next_page_token.is_some()
    }
}

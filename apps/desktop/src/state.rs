use mutsukitube_core::Video;

#[derive(Default)]
pub struct AppState {
    pub search_results: Vec<Video>,

    pub history_results: Vec<Video>,

    pub current_query: String,

    pub continuation_token: Option<String>,

    pub search_generation: u64,

    pub loading_more: bool,

    pub provider_mode: String,

    pub library_results: Vec<Video>,
}

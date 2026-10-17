use mutsukitube_anime::{Anime, Episode};
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

    pub history_generation: u64,

    pub library_generation: u64,

    pub anime_results: Vec<Anime>,

    pub anime_episodes: Vec<Episode>,

    pub selected_anime: Option<Anime>,

    pub selected_episode: Option<Episode>,

    pub anime_generation: u64,

    pub anime_loading: bool,

    pub anime_qualities: Vec<String>,
}

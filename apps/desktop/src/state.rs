use std::collections::HashMap;

use mutsukitube_anime::{Anime, Episode};
use mutsukitube_core::Video;

/// Durable identity of the anime episode currently loaded in the player.
/// Streaming URLs are never stored here; only provider identity and episode
/// number, so Continue Watching can resolve the episode again later.
#[derive(Debug, Clone)]
pub struct AnimePlaying {
    pub provider_id: String,
    pub anime_url: String,
    pub title: String,
    pub thumbnail_url: String,
    pub episode_number: i32,
    pub episode_title: String,
}

#[derive(Default)]
pub struct AppState {
    pub search_results: Vec<Video>,

    pub home_results: Vec<Video>,

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

    // ---- Anime library (Phase 5.5) -----------------------------------------
    /// "home" | "details" | "setup"
    pub anime_view: String,

    pub anime_selected_key: String,

    /// Lookup for every card currently displayed (home shelves + results).
    pub anime_cards: HashMap<String, Anime>,

    /// Resume position per anime key: (episode_number, position_seconds).
    pub anime_resume: HashMap<String, (i32, u64)>,

    pub anime_selected_episode_number: i32,

    pub anime_now_playing: Option<AnimePlaying>,

    /// Episode whose playback was just resolved and is about to be loaded in the
    /// player. Promoted to `anime_now_playing` only when the media actually
    /// loads, so progress is never attributed to an episode that is not playing.
    pub anime_pending_playing: Option<AnimePlaying>,

    pub anime_pending_resume: Option<u64>,

    /// Last unix second we persisted playback progress.
    pub anime_last_progress_save: u64,

    // ---- Navigation stack (Phase 5.5.2) --------------------------------------
    /// Back navigation history. The top entry is the currently shown screen;
    /// popping it restores the screen the user came from.
    pub nav_stack: Vec<crate::navigation::NavEntry>,

    // ---- YouTube Home feed (Phase 5.5.2) -------------------------------------
    pub home_topic: String,
    pub home_loading: bool,
    pub home_status: String,
    pub home_generation: u64,

    // ---- YouTube Trending (Option A) ----------------------------------------
    pub trending_results: Vec<Video>,
    pub trending_category: String,
    pub trending_country: String,
    pub trending_loading: bool,
    pub trending_status: String,
    pub trending_generation: u64,

    // ---- YouTube Subscriptions ----------------------------------------------
    pub subscriptions_all: Vec<mutsukitube_storage::SubscribedChannel>,
    pub subscriptions_filtered: Vec<mutsukitube_storage::SubscribedChannel>,
    pub subscriptions_channel_videos: Vec<Video>,
    pub subscriptions_selected_channel_id: Option<String>,
    pub subscriptions_selected_channel_title: Option<String>,
    pub subscriptions_filter_query: String,
    pub subscriptions_current_page: usize,
    pub subscriptions_sort: String,
    pub subscriptions_loading: bool,
    pub subscriptions_status: String,
    pub subscriptions_generation: u64,
}

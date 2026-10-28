pub mod anime;
pub mod database;
pub mod history;
pub mod importer;
pub mod library;
pub mod settings;

pub use database::{database_path, open_connection};

pub use anime::{
    AnimeEntry, AnimeHistoryRecord, AnimeProgressRecord, WATCHED_COMPLETION_THRESHOLD, anime_key,
    get_anime_favorites, get_anime_history, get_anime_progress_for, get_continue_watching,
    is_anime_favorite, save_anime_progress, toggle_anime_favorite,
};

pub use history::{clear_history, get_history, save_video};

pub use importer::{
    SubscribedChannel, add_subscription, clear_subscriptions, get_subscriptions,
    get_subscriptions_count, import_subscriptions, is_subscribed, parse_subscriptions_csv,
    remove_subscription, update_channel_avatar,
};

pub use settings::{get_provider_mode, set_provider_mode, validate_provider_mode};

pub use library::{
    add_to_playlist, create_playlist, get_favorites, get_playlist_names, get_playlist_videos,
    is_favorite, toggle_favorite,
};

#[cfg(test)]
mod issue_simulations;

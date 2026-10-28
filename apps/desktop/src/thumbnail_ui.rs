use std::sync::{Arc, Mutex};

use futures::stream::{self, StreamExt};

use mutsukitube_core::Video;

use slint::{Model, ModelRc};

use crate::{AppWindow, VideoItem, state::AppState, thumbnail::download_thumbnail};

#[derive(Debug, Clone, Copy)]
pub enum ThumbnailTarget {
    Search,
    History,
    Library,
    Home,
    HomeContinue,
    HomeFavorites,
    Trending,
    Subscriptions,
}

fn get_model(ui: &AppWindow, target: ThumbnailTarget) -> ModelRc<VideoItem> {
    match target {
        ThumbnailTarget::Search => ui.get_videos(),
        ThumbnailTarget::History => ui.get_history_videos(),
        ThumbnailTarget::Library => ui.get_library_videos(),
        ThumbnailTarget::Home => ui.get_home_videos(),
        ThumbnailTarget::HomeContinue => ui.get_home_continue_videos(),
        ThumbnailTarget::HomeFavorites => ui.get_home_favorite_videos(),
        ThumbnailTarget::Trending => ui.get_trending_videos(),
        ThumbnailTarget::Subscriptions => ui.get_subscriptions_channel_videos(),
    }
}

fn update_thumbnail(ui: &AppWindow, target: ThumbnailTarget, video_id: &str, bytes: &[u8]) {
    let Ok(image) = slint::Image::load_from_data(bytes, None) else {
        return;
    };

    let model = get_model(ui, target);

    for index in 0..model.row_count() {
        let Some(mut item) = model.row_data(index) else {
            continue;
        };

        if item.id.as_str() != video_id {
            continue;
        }

        item.thumbnail = image;

        model.set_row_data(index, item);

        break;
    }
}

pub fn queue_thumbnails(
    videos: Vec<Video>,
    target: ThumbnailTarget,
    generation: u64,
    weak: slint::Weak<AppWindow>,
    state: Arc<Mutex<AppState>>,
    handle: tokio::runtime::Handle,
) {
    handle.spawn(async move {
        stream::iter(videos)
            .for_each_concurrent(6, |video| {
                let weak = weak.clone();
                let state = state.clone();

                async move {
                    let Some(bytes) = download_thumbnail(&video.thumbnail).await else {
                        return;
                    };

                    let video_id = video.id;

                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        let valid = state
                            .lock()
                            .map(|s| match target {
                                ThumbnailTarget::Search => s.search_generation == generation,

                                ThumbnailTarget::History => s.history_generation == generation,

                                ThumbnailTarget::Library => s.library_generation == generation,

                                ThumbnailTarget::Home
                                | ThumbnailTarget::HomeContinue
                                | ThumbnailTarget::HomeFavorites => s.home_generation == generation,

                                ThumbnailTarget::Trending => s.trending_generation == generation,

                                ThumbnailTarget::Subscriptions => {
                                    s.subscriptions_generation == generation
                                }
                            })
                            .unwrap_or(false);

                        if !valid {
                            return;
                        }

                        update_thumbnail(&ui, target, &video_id, &bytes);
                    });
                }
            })
            .await;
    });
}

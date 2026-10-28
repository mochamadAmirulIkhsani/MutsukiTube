use std::rc::Rc;

use mutsukitube_core::{Video, VideoProvider};
use mutsukitube_storage::{get_favorites, get_history};
use mutsukitube_youtube::YoutubeProvider;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, VideoItem,
    app_context::AppContext,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::video_to_ui,
};

/// Bounded number of explore videos per topic so startup never bursts the
/// provider with dozens of simultaneous searches.
const EXPLORE_LIMIT: usize = 24;

fn replace_home_videos(ui: &AppWindow, items: Vec<VideoItem>) {
    let model = ui.get_home_videos();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<crate::VideoItem>>() {
        model.set_vec(items);
    } else {
        ui.set_home_videos(ModelRc::from(Rc::new(VecModel::from(items))));
    }
}

fn load_home(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.home_topic = String::new();
        state.home_generation = state.home_generation.wrapping_add(1);
        state.home_loading = true;
        state.home_status = "Loading home...".into();
        state.home_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_home_selected_topic("".into());
        ui.set_home_loading(true);
        ui.set_home_status_text("Loading home...".into());
    }

    let handle = context.handle.clone();
    let spawn_handle = handle.clone();

    spawn_handle.spawn(async move {
        let history = tokio::task::spawn_blocking(|| get_history(EXPLORE_LIMIT))
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();

        let favorites = tokio::task::spawn_blocking(|| get_favorites())
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();

        let provider = YoutubeProvider::new();
        let explore = provider
            .search("Music")
            .await
            .ok()
            .unwrap_or_default()
            .into_iter()
            .take(EXPLORE_LIMIT)
            .collect::<Vec<Video>>();

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.home_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            let continue_items: Vec<VideoItem> = history
                .iter()
                .map(|video: &Video| video_to_ui(video, slint::Image::default()))
                .collect();

            let favorite_items: Vec<VideoItem> = favorites
                .iter()
                .map(|video: &Video| video_to_ui(video, slint::Image::default()))
                .collect();

            let explore_items: Vec<VideoItem> = explore
                .iter()
                .map(|video: &Video| video_to_ui(video, slint::Image::default()))
                .collect();

            if let Ok(mut state) = context.state.lock() {
                state.home_loading = false;
                state.home_status = format!("{} videos", explore_items.len());
                state.home_results = explore.clone();
                for v in &history {
                    if !state.history_results.iter().any(|existing| existing.id == v.id) {
                        state.history_results.push(v.clone());
                    }
                }
                for v in &favorites {
                    if !state.library_results.iter().any(|existing| existing.id == v.id) {
                        state.library_results.push(v.clone());
                    }
                }
            }

            ui.set_home_loading(false);
            ui.set_home_status_text(format!("{} videos", explore_items.len()).into());

            ui.set_home_continue_videos(ModelRc::from(Rc::new(VecModel::from(continue_items))));
            ui.set_home_favorite_videos(ModelRc::from(Rc::new(VecModel::from(favorite_items))));
            replace_home_videos(&ui, explore_items);

            queue_thumbnails(
                explore,
                ThumbnailTarget::Home,
                generation,
                ui.as_weak(),
                context.state.clone(),
                handle.clone(),
            );
            queue_thumbnails(
                history,
                ThumbnailTarget::HomeContinue,
                generation,
                ui.as_weak(),
                context.state.clone(),
                handle.clone(),
            );
            queue_thumbnails(
                favorites,
                ThumbnailTarget::HomeFavorites,
                generation,
                ui.as_weak(),
                context.state.clone(),
                handle,
            );
        });
    });
}

fn load_topic(weak: slint::Weak<AppWindow>, context: AppContext, topic: String) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.home_topic = topic.clone();
        state.home_generation = state.home_generation.wrapping_add(1);
        state.home_loading = true;
        state.home_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_home_selected_topic(topic.clone().into());
        ui.set_home_loading(true);
        ui.set_home_status_text("".into());
        replace_home_videos(&ui, Vec::new());
    }

    let handle = context.handle.clone();
    let spawn_handle = handle.clone();

    spawn_handle.spawn(async move {
        let provider = YoutubeProvider::new();
        let result = provider.search(&topic).await;

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.home_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            ui.set_home_loading(false);

            match result {
                Ok(videos) => {
                    let page = videos
                        .into_iter()
                        .take(EXPLORE_LIMIT)
                        .collect::<Vec<Video>>();

                    let items: Vec<VideoItem> = page
                        .iter()
                        .map(|video: &Video| video_to_ui(video, slint::Image::default()))
                        .collect();

                    if let Ok(mut state) = context.state.lock() {
                        state.home_status = format!("{} videos", items.len());
                        state.home_results = page.clone();
                    }

                    ui.set_home_status_text(format!("{} videos", items.len()).into());
                    replace_home_videos(&ui, items);

                    queue_thumbnails(
                        page,
                        ThumbnailTarget::Home,
                        generation,
                        ui.as_weak(),
                        context.state.clone(),
                        handle,
                    );
                }
                Err(error) => {
                    ui.set_home_status_text(format!("Explore failed: {error}").into());
                }
            }
        });
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_load_home(move || {
            load_home(weak.clone(), context.clone());
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_load_home_topic(move |topic| {
            let topic = topic.trim().to_string();

            if topic.is_empty() {
                load_home(weak.clone(), context.clone());
                return;
            }

            load_topic(weak.clone(), context.clone(), topic);
        });
    }
}

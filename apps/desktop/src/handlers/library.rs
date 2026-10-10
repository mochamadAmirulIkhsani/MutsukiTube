use std::rc::Rc;

use mutsukitube_core::Video;

use mutsukitube_storage::{
    add_to_playlist, create_playlist, get_favorites, get_playlist_names, get_playlist_videos,
    toggle_favorite,
};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow,
    app_context::AppContext,
    state::AppState,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::video_to_ui,
};

fn find_video(state: &AppState, video_id: &str) -> Option<Video> {
    state
        .search_results
        .iter()
        .chain(state.history_results.iter())
        .chain(state.library_results.iter())
        .find(|video| video.id == video_id)
        .cloned()
}

fn refresh_library(
    weak: slint::Weak<AppWindow>,
    context: AppContext,
    mode: String,
    playlist: String,
) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.library_generation = state.library_generation.wrapping_add(1);

        state.library_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_library_loading(true);
        ui.set_library_status("Loading library...".into());
    }

    let handle = context.handle.clone();

    handle.spawn_blocking(move || {
        let result = (|| {
            let names = get_playlist_names()?;

            let videos = if mode == "favorites" {
                get_favorites()?
            } else {
                get_playlist_videos(&playlist)?
            };

            Ok::<_, rusqlite::Error>((names, videos))
        })();

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.library_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            if ui.get_library_mode().as_str() != mode {
                return;
            }

            if mode == "playlist" && ui.get_selected_playlist().as_str() != playlist {
                return;
            }

            ui.set_library_loading(false);

            match result {
                Ok((names, videos)) => {
                    let names = names
                        .into_iter()
                        .map(SharedString::from)
                        .collect::<Vec<_>>();

                    ui.set_playlist_names(ModelRc::from(Rc::new(VecModel::from(names))));

                    let count = videos.len();

                    if let Ok(mut state) = context.state.lock() {
                        state.library_results = videos.clone();
                    }

                    let items = videos
                        .iter()
                        .map(|video| video_to_ui(video, slint::Image::default()))
                        .collect::<Vec<_>>();

                    ui.set_library_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                    ui.set_library_status(format!("{count} videos").into());

                    queue_thumbnails(
                        videos,
                        ThumbnailTarget::Library,
                        generation,
                        ui.as_weak(),
                        context.state.clone(),
                        context.handle.clone(),
                    );
                }

                Err(error) => {
                    ui.set_library_status(format!("Library error: {error}").into());
                }
            }
        });
    });
}

fn register_navigation(ui: &AppWindow, context: &AppContext) {
    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_show_library(move || {
            refresh_library(
                weak.clone(),
                context.clone(),
                "favorites".to_string(),
                String::new(),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_refresh_library(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let mode = ui.get_library_mode().to_string();
            let playlist = ui.get_selected_playlist().to_string();

            refresh_library(weak.clone(), context.clone(), mode, playlist);
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_select_playlist(move |name| {
            refresh_library(
                weak.clone(),
                context.clone(),
                "playlist".to_string(),
                name.to_string(),
            );
        });
    }
}

fn register_create_playlist(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_create_playlist(move |name| {
        let name = name.trim().to_string();

        if name.is_empty() {
            if let Some(ui) = weak.upgrade() {
                ui.set_library_status("Playlist name cannot be empty".into());
            }
            return;
        }

        if let Some(ui) = weak.upgrade() {
            ui.set_library_status("Creating playlist...".into());
        }

        let weak = weak.clone();
        let context_for_refresh = context.clone();

        context.handle.spawn_blocking(move || {
            let result = create_playlist(&name);

            let _ = weak.upgrade_in_event_loop(move |ui| match result {
                Ok(()) => {
                    ui.set_library_mode("playlist".into());
                    ui.set_selected_playlist(name.clone().into());

                    refresh_library(
                        ui.as_weak(),
                        context_for_refresh,
                        "playlist".to_string(),
                        name,
                    );
                }

                Err(error) => {
                    ui.set_library_status(format!("Failed to create playlist: {error}").into());
                }
            });
        });
    });
}

fn register_toggle_favorite(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_toggle_favorite(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        let video_id = ui.get_selected_video_id().to_string();

        let video = context
            .state
            .lock()
            .ok()
            .and_then(|state| find_video(&state, &video_id));

        let Some(video) = video else {
            ui.set_watch_library_status("Video not found".into());
            return;
        };

        ui.set_watch_library_status("Updating Favorites...".into());

        let weak = weak.clone();

        context.handle.spawn_blocking(move || {
            let result = toggle_favorite(&video);

            let _ = weak.upgrade_in_event_loop(move |ui| {
                if ui.get_selected_video_id().as_str() != video_id {
                    return;
                }

                match result {
                    Ok(true) => {
                        ui.set_watch_library_status("Added to Favorites".into());
                    }

                    Ok(false) => {
                        ui.set_watch_library_status("Removed from Favorites".into());
                    }

                    Err(error) => {
                        ui.set_watch_library_status(format!("Favorite error: {error}").into());
                    }
                }
            });
        });
    });
}

fn register_add_to_playlist(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_add_to_playlist(move |name| {
        let playlist_name = name.to_string();

        let Some(ui) = weak.upgrade() else {
            return;
        };

        if playlist_name.trim().is_empty() {
            ui.set_watch_library_status("Select a playlist first".into());
            return;
        }

        let video_id = ui.get_selected_video_id().to_string();

        let video = context
            .state
            .lock()
            .ok()
            .and_then(|state| find_video(&state, &video_id));

        let Some(video) = video else {
            ui.set_watch_library_status("Video not found".into());
            return;
        };

        ui.set_watch_library_status("Adding to playlist...".into());

        let weak = weak.clone();

        context.handle.spawn_blocking(move || {
            let result = add_to_playlist(&playlist_name, &video);

            let _ = weak.upgrade_in_event_loop(move |ui| {
                if ui.get_selected_video_id().as_str() != video_id {
                    return;
                }

                match result {
                    Ok(()) => {
                        ui.set_watch_library_status("Video saved to playlist".into());
                    }

                    Err(error) => {
                        ui.set_watch_library_status(format!("Playlist error: {error}").into());
                    }
                }
            });
        });
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    register_navigation(ui, context);
    register_create_playlist(ui, context);
    register_toggle_favorite(ui, context);
    register_add_to_playlist(ui, context);
}

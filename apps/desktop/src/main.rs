mod app_context;
mod handlers;
mod state;
mod thumbnail;
mod thumbnail_ui;
mod ui_models;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use mutsukitube_core::Video;

use mutsukitube_storage::{
    add_to_playlist, get_playlist_names, get_provider_mode, toggle_favorite,
};

use slint::{ComponentHandle, ModelRc, VecModel};

use state::AppState;

use app_context::AppContext;

slint::include_modules!();

fn find_video(state: &AppState, id: &str) -> Option<Video> {
    state
        .search_results
        .iter()
        .chain(state.history_results.iter())
        .chain(state.library_results.iter())
        .find(|video| video.id == id)
        .cloned()
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let runtime = Rc::new(tokio::runtime::Runtime::new().expect("failed to create Tokio runtime"));

    let tokio_handle = runtime.handle().clone();

    let saved_provider = match get_provider_mode() {
        Ok(mode) => mode,

        Err(error) => {
            eprintln!("[MutsukiTube] Failed to load settings: {error}");

            "auto".to_string()
        }
    };

    let state = Arc::new(Mutex::new(AppState {
        provider_mode: saved_provider.clone(),
        ..AppState::default()
    }));

    let context = AppContext {
        state: state.clone(),
        handle: tokio_handle.clone(),
    };

    ui.set_provider_mode(saved_provider.into());

    let video_model = Rc::new(VecModel::<VideoItem>::default());

    ui.set_videos(ModelRc::from(video_model));

    // ========================================
    // TOGGLE FAVORITE
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_toggle_favorite(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let video_id = ui.get_selected_video_id().to_string();

            let video = state.lock().ok().and_then(|s| find_video(&s, &video_id));

            let Some(video) = video else {
                ui.set_watch_library_status("Video not found".into());
                return;
            };

            ui.set_watch_library_status("Saving favorite...".into());

            let weak = weak.clone();

            handle.spawn_blocking(move || {
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

    // ========================================
    // ADD VIDEO TO PLAYLIST
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_add_to_playlist(move |name| {
            let playlist_name = name.to_string();

            let Some(ui) = weak.upgrade() else {
                return;
            };

            if playlist_name.is_empty() {
                ui.set_watch_library_status("Select a playlist first".into());
                return;
            }

            let video_id = ui.get_selected_video_id().to_string();

            let video = state.lock().ok().and_then(|s| find_video(&s, &video_id));

            let Some(video) = video else {
                ui.set_watch_library_status("Video not found".into());
                return;
            };

            ui.set_watch_library_status("Adding to playlist...".into());

            let weak = weak.clone();

            handle.spawn_blocking(move || {
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

    match get_playlist_names() {
        Ok(names) => {
            let items = names
                .into_iter()
                .map(slint::SharedString::from)
                .collect::<Vec<_>>();

            ui.set_playlist_names(ModelRc::from(Rc::new(VecModel::from(items))));
        }

        Err(error) => {
            eprintln!("[MutsukiTube] Failed to load playlists: {error}");
        }
    }

    handlers::player::register(&ui, &context);

    handlers::history::register(&ui, &context);

    handlers::settings::register(&ui, &context);

    handlers::library::register(&ui, &context);

    handlers::search::register(&ui, &context);

    ui.run()
}

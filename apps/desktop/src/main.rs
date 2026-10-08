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

use mutsukitube_core::{ProviderError, Video, VideoProvider};

use mutsukitube_storage::{
    add_to_playlist, create_playlist, get_favorites, get_history, get_playlist_names,
    get_playlist_videos, get_provider_mode, set_provider_mode, toggle_favorite,
};

use mutsukitube_youtube::{NativeYoutubeProvider, ProviderMode, SearchPage, YtDlpProvider};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use state::AppState;

use rusqlite;

use ui_models::video_to_ui;

use thumbnail_ui::{ThumbnailTarget, queue_thumbnails};

use app_context::AppContext;

slint::include_modules!();

fn parse_provider_mode(value: &str) -> ProviderMode {
    match value {
        "native" => ProviderMode::Native,
        "ytdlp" => ProviderMode::YtDlp,
        _ => ProviderMode::Auto,
    }
}

// Provider untuk halaman pertama.
// Mode auto mencoba native lalu fallback.
async fn fetch_first_page(query: &str, mode: ProviderMode) -> Result<SearchPage, ProviderError> {
    match mode {
        ProviderMode::Native => NativeYoutubeProvider::new().search_page(query, None).await,

        ProviderMode::YtDlp => {
            let videos = YtDlpProvider::new().search(query).await?;

            Ok(SearchPage {
                videos,
                next_page_token: None,
            })
        }

        ProviderMode::Auto => match NativeYoutubeProvider::new().search_page(query, None).await {
            Ok(page) if !page.videos.is_empty() => {
                println!("[MutsukiTube] Native: {} results", page.videos.len());
                Ok(page)
            }

            Ok(_) | Err(_) => {
                eprintln!("[MutsukiTube] Falling back to yt-dlp");

                let videos = YtDlpProvider::new().search(query).await?;

                Ok(SearchPage {
                    videos,
                    next_page_token: None,
                })
            }
        },
    }
}

fn load_history(
    weak: slint::Weak<AppWindow>,
    state: Arc<Mutex<AppState>>,
    handle: tokio::runtime::Handle,
) {
    if let Some(ui) = weak.upgrade() {
        ui.set_history_loading(true);
        ui.set_history_status("Loading history...".into());
    }

    let generation = {
        let mut s = state.lock().unwrap();

        s.history_generation = s.history_generation.wrapping_add(1);

        s.history_generation
    };

    let handle_for_images = handle.clone();

    handle.spawn(async move {
        let result = tokio::task::spawn_blocking(|| get_history(100)).await;

        let _ = weak.upgrade_in_event_loop(move |ui| {
            ui.set_history_loading(false);

            match result {
                Ok(Ok(videos)) => {
                    let is_current = state
                        .lock()
                        .map(|s| s.history_generation == generation)
                        .unwrap_or(false);

                    if !is_current {
                        return;
                    }
                    let count = videos.len();

                    if let Ok(mut s) = state.lock() {
                        s.history_results = videos.clone();
                    }

                    let items = videos
                        .iter()
                        .map(|video| video_to_ui(video, slint::Image::default()))
                        .collect::<Vec<_>>();

                    ui.set_history_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                    queue_thumbnails(
                        videos,
                        ThumbnailTarget::History,
                        generation,
                        ui.as_weak(),
                        state.clone(),
                        handle_for_images,
                    );

                    ui.set_history_status(format!("{count} videos in history").into());
                }

                Ok(Err(error)) => {
                    ui.set_history_status(format!("Database error: {error}").into());
                }

                Err(error) => {
                    ui.set_history_status(format!("History task failed: {error}").into());
                }
            }
        });
    });
}

fn find_video(state: &AppState, id: &str) -> Option<Video> {
    state
        .search_results
        .iter()
        .chain(state.history_results.iter())
        .chain(state.library_results.iter())
        .find(|video| video.id == id)
        .cloned()
}

fn refresh_library(
    weak: slint::Weak<AppWindow>,
    state: Arc<Mutex<AppState>>,
    handle: tokio::runtime::Handle,
    mode: String,
    playlist: String,
) {
    if let Some(ui) = weak.upgrade() {
        ui.set_library_loading(true);
        ui.set_library_status("Loading library...".into());
    }

    let generation = {
        let mut s = state.lock().unwrap();

        s.library_generation = s.library_generation.wrapping_add(1);

        s.library_generation
    };

    let handle_for_images = handle.clone();

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
            let is_current = state
                .lock()
                .map(|s| s.library_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            // Jangan tampilkan hasil request yang sudah tidak relevan.
            if ui.get_library_mode().as_str() != mode
                || (mode == "playlist" && ui.get_selected_playlist().as_str() != playlist)
            {
                return;
            }

            ui.set_library_loading(false);

            match result {
                Ok((names, videos)) => {
                    let names = names
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>();

                    ui.set_playlist_names(ModelRc::from(Rc::new(VecModel::from(names))));

                    let count = videos.len();

                    if let Ok(mut s) = state.lock() {
                        s.library_results = videos.clone();
                    }

                    let items = videos
                        .iter()
                        .map(|video| video_to_ui(video, slint::Image::default()))
                        .collect::<Vec<_>>();

                    ui.set_library_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                    queue_thumbnails(
                        videos,
                        ThumbnailTarget::Library,
                        generation,
                        ui.as_weak(),
                        state.clone(),
                        handle_for_images,
                    );

                    ui.set_library_status(format!("{count} videos").into());
                }

                Err(error) => {
                    ui.set_library_status(format!("Library error: {error}").into());
                }
            }
        });
    });
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
    // SEARCH — FIRST PAGE
    // ========================================
    {
        let weak = ui.as_weak();
        let runtime = runtime.clone();
        let state = state.clone();
        let tokio_handle = tokio_handle.clone();

        ui.on_search(move |query| {
            let query = query.trim().to_string();

            if query.is_empty() {
                return;
            }

            let (generation, provider_mode) = {
                let mut s = state.lock().unwrap();

                s.search_generation = s.search_generation.wrapping_add(1);

                s.current_query = query.clone();
                s.search_results.clear();
                s.continuation_token = None;
                s.loading_more = false;

                let provider_mode = parse_provider_mode(&s.provider_mode);

                (s.search_generation, provider_mode)
            };

            if let Some(ui) = weak.upgrade() {
                ui.set_loading(true);
                ui.set_loading_more(false);
                ui.set_has_more(false);
                ui.set_status_text("".into());

                let model = ui.get_videos();

                let model = model
                    .as_any()
                    .downcast_ref::<VecModel<VideoItem>>()
                    .expect("Expected VecModel");

                model.clear();
            }

            let weak = weak.clone();
            let state = state.clone();
            let handle_for_images = tokio_handle.clone();

            runtime.spawn(async move {
                let result = fetch_first_page(&query, provider_mode).await;

                match result {
                    Ok(page) => {
                        let videos = page.videos.clone();
                        let next_token = page.next_page_token;
                        let count = videos.len();

                        let weak_for_images = weak.clone();
                        let state_for_images = state.clone();

                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.search_results = videos.clone();
                            s.continuation_token = next_token;

                            let has_more = s.continuation_token.is_some();

                            drop(s);

                            let items = videos
                                .iter()
                                .map(|video| video_to_ui(video, slint::Image::default()))
                                .collect::<Vec<_>>();

                            let model = ui.get_videos();

                            let model = model
                                .as_any()
                                .downcast_ref::<VecModel<VideoItem>>()
                                .expect("Expected VecModel");

                            model.set_vec(items);

                            ui.set_loading(false);
                            ui.set_has_more(has_more);
                            ui.set_status_text(format!("{count} videos found").into());

                            queue_thumbnails(
                                videos,
                                ThumbnailTarget::Search,
                                generation,
                                weak_for_images,
                                state_for_images,
                                handle_for_images,
                            );
                        });
                    }

                    Err(error) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let current = state
                                .lock()
                                .map(|s| s.search_generation)
                                .unwrap_or_default();

                            if current != generation {
                                return;
                            }

                            ui.set_loading(false);
                            ui.set_status_text(format!("Search failed: {error}").into());
                        });
                    }
                }
            });
        });
    }

    // ========================================
    // LOAD MORE — NEXT PAGE
    // ========================================
    {
        let weak = ui.as_weak();
        let runtime = runtime.clone();
        let state = state.clone();
        let tokio_handle = tokio_handle.clone();

        ui.on_load_more(move || {
            let request = {
                let mut s = state.lock().unwrap();

                if s.loading_more {
                    None
                } else {
                    s.continuation_token.clone().map(|token| {
                        s.loading_more = true;

                        (s.current_query.clone(), token, s.search_generation)
                    })
                }
            };

            let Some((query, token, generation)) = request else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                ui.set_loading_more(true);
            }

            let weak = weak.clone();
            let state = state.clone();
            let handle_for_images = tokio_handle.clone();

            runtime.spawn(async move {
                let result = NativeYoutubeProvider::new()
                    .search_page(&query, Some(&token))
                    .await;

                match result {
                    Ok(page) => {
                        let weak_for_images = weak.clone();
                        let state_for_images = state.clone();

                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.loading_more = false;

                            // Hindari duplikasi video jika
                            // halaman mengandung ID yang sama.
                            let new_videos = page
                                .videos
                                .into_iter()
                                .filter(|video| {
                                    !s.search_results
                                        .iter()
                                        .any(|existing| existing.id == video.id)
                                })
                                .collect::<Vec<_>>();

                            let has_new = !new_videos.is_empty();

                            // Hindari continuation yang berulang.
                            s.continuation_token = if has_new
                                && page.next_page_token.as_deref() != Some(token.as_str())
                            {
                                page.next_page_token
                            } else {
                                None
                            };

                            s.search_results.extend(new_videos.iter().cloned());

                            let total = s.search_results.len();

                            let has_more = s.continuation_token.is_some();

                            drop(s);

                            let items = new_videos
                                .iter()
                                .map(|video| video_to_ui(video, slint::Image::default()))
                                .collect::<Vec<_>>();

                            let model = ui.get_videos();

                            let model = model
                                .as_any()
                                .downcast_ref::<VecModel<VideoItem>>()
                                .expect("Expected VecModel");

                            model.extend(items);

                            ui.set_loading_more(false);
                            ui.set_has_more(has_more);
                            ui.set_status_text(format!("{total} videos found").into());

                            queue_thumbnails(
                                new_videos,
                                ThumbnailTarget::Search,
                                generation,
                                weak_for_images,
                                state_for_images,
                                handle_for_images,
                            );
                        });
                    }

                    Err(error) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.loading_more = false;

                            ui.set_loading_more(false);
                            ui.set_status_text(format!("Load More failed: {error}").into());
                        });
                    }
                }
            });
        });
    }

    // ========================================
    // SHOW HISTORY
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_show_history(move || {
            load_history(weak.clone(), state.clone(), handle.clone());
        });
    }

    // ========================================
    // REFRESH HISTORY
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_refresh_history(move || {
            load_history(weak.clone(), state.clone(), handle.clone());
        });
    }

    // ========================================
    // SAVE SETTINGS
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_save_provider(move |mode| {
            let mode = mode.to_string();

            if !matches!(mode.as_str(), "auto" | "native" | "ytdlp") {
                if let Some(ui) = weak.upgrade() {
                    ui.set_settings_status("Invalid provider mode".into());
                }

                return;
            }

            if let Some(ui) = weak.upgrade() {
                ui.set_settings_status("Saving settings...".into());
            }

            let weak = weak.clone();
            let state = state.clone();

            handle.spawn_blocking(move || {
                let result = set_provider_mode(&mode);

                let _ = weak.upgrade_in_event_loop(move |ui| {
                    match result {
                        Ok(()) => {
                            if let Ok(mut s) = state.lock() {
                                s.provider_mode = mode.clone();

                                // Batalkan hasil request lama
                                // setelah provider berganti.
                                s.search_generation = s.search_generation.wrapping_add(1);

                                s.loading_more = false;
                                s.continuation_token = None;
                            }

                            ui.set_provider_mode(mode.into());

                            ui.set_loading(false);
                            ui.set_loading_more(false);
                            ui.set_has_more(false);

                            ui.set_settings_status("Settings saved successfully".into());
                        }

                        Err(error) => {
                            ui.set_settings_status(
                                format!("Failed to save settings: {error}").into(),
                            );
                        }
                    }
                });
            });
        });
    }

    // ========================================
    // OPEN LIBRARY
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_show_library(move || {
            refresh_library(
                weak.clone(),
                state.clone(),
                handle.clone(),
                "favorites".to_string(),
                String::new(),
            );
        });
    }

    // ========================================
    // REFRESH LIBRARY
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_refresh_library(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let mode = ui.get_library_mode().to_string();
            let playlist = ui.get_selected_playlist().to_string();

            refresh_library(weak.clone(), state.clone(), handle.clone(), mode, playlist);
        });
    }

    // ========================================
    // SELECT PLAYLIST
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_select_playlist(move |name| {
            refresh_library(
                weak.clone(),
                state.clone(),
                handle.clone(),
                "playlist".to_string(),
                name.to_string(),
            );
        });
    }

    // ========================================
    // CREATE PLAYLIST
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();
        let handle = tokio_handle.clone();

        ui.on_create_playlist(move |name| {
            let name = name.trim().to_string();

            if name.is_empty() {
                if let Some(ui) = weak.upgrade() {
                    ui.set_library_status("Playlist name cannot be empty".into());
                }
                return;
            }

            let weak = weak.clone();
            let state = state.clone();
            let handle_for_refresh = handle.clone();

            handle.spawn_blocking(move || {
                let result = create_playlist(&name);

                let _ = weak.upgrade_in_event_loop(move |ui| match result {
                    Ok(()) => {
                        ui.set_library_mode("playlist".into());
                        ui.set_selected_playlist(name.clone().into());

                        refresh_library(
                            ui.as_weak(),
                            state,
                            handle_for_refresh,
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

    ui.run()
}

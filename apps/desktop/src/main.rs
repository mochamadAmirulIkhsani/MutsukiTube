mod app_context;
mod handlers;
mod native_window;
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

#[cfg(target_os = "windows")]
use std::cell::RefCell;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(target_os = "windows")]
use native_window::windows::Win32VideoSurface;

#[cfg(target_os = "windows")]
use native_window::session::NativeVideoSession;

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

#[cfg(target_os = "windows")]
fn get_slint_hwnd(ui: &AppWindow) -> Option<windows_sys::Win32::Foundation::HWND> {
    let window = ui.window().window_handle();

    let handle = window.window_handle().ok()?;

    match handle.as_raw() {
        RawWindowHandle::Win32(win32) => Some(win32.hwnd.get() as *mut std::ffi::c_void),

        _ => None,
    }
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

    #[cfg(target_os = "windows")]
    let (surface_timer, native_session) = {
        use std::time::Duration;

        let weak = ui.as_weak();

        use std::cell::Cell;

        let paused_for_navigation = Cell::new(false);

        let session = Rc::new(RefCell::new(NativeVideoSession::new()));

        let session_for_timer = session.clone();

        let test_video = std::env::var("MUTSUKITUBE_TEST_VIDEO").ok();

        let timer = slint::Timer::default();

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(100),
            move || {
                let Some(ui) = weak.upgrade() else {
                    return;
                };

                let is_watch = ui.get_current_page().as_str() == "watch";

                let mut session = session_for_timer.borrow_mut();

                // ======================================
                // CREATE NATIVE VIDEO SURFACE
                // ======================================

                if is_watch && session.surface.is_none() {
                    let Some(parent) = get_slint_hwnd(&ui) else {
                        return;
                    };

                    match Win32VideoSurface::new(parent) {
                        Ok(surface) => {
                            println!("[MutsukiTube] Child HWND created");

                            session.surface = Some(surface);
                        }

                        Err(error) => {
                            eprintln!("[MutsukiTube] Surface error: {error}");

                            return;
                        }
                    }
                }

                // ======================================
                // UPDATE SURFACE
                // ======================================

                if let Some(surface) = session.surface.as_ref() {
                    if is_watch {
                        if let Some(surface) = session.surface.as_ref() {
                            if is_watch {
                                if let Err(error) = surface.set_geometry(24, 100, 640, 360) {
                                    eprintln!("[MutsukiTube] Geometry error: {error}");
                                }

                                surface.show();
                            } else {
                                surface.hide();
                                if !paused_for_navigation.get() {
                                    if let Some(player) = session.player.as_ref() {
                                        match player.pause() {
                                            Ok(()) => {
                                                paused_for_navigation.set(true);

                                                println!(
                                                    "[MutsukiTube] Playback paused: \
                                                    leaving WatchPage"
                                                );
                                            }

                                            Err(error) => {
                                                eprintln!("[MutsukiTube] Pause error: {error}");
                                            }
                                        }
                                    }
                                }

                                return;
                            }
                        }

                        surface.show();
                    } else {
                        surface.hide();
                        return;
                    }
                }

                // ======================================
                // INITIALIZE LIBMPV
                // ======================================

                if is_watch && session.player.is_none() {
                    let Some(surface) = session.surface.as_ref() else {
                        return;
                    };

                    let hwnd = surface.hwnd() as usize;

                    match mutsukitube_embedded_player::EmbeddedMpvPlayer::new_for_hwnd(hwnd) {
                        Ok(player) => {
                            println!("[MutsukiTube] Embedded libmpv initialized");

                            session.player = Some(player);
                        }

                        Err(error) => {
                            eprintln!("[MutsukiTube] libmpv error: {error}");

                            return;
                        }
                    }
                }

                // ======================================
                // RESUME AFTER NAVIGATION
                // ======================================

                if paused_for_navigation.get() {
                    if let Some(player) = session.player.as_ref() {
                        match player.play() {
                            Ok(()) => {
                                paused_for_navigation.set(false);

                                println!(
                                    "[MutsukiTube] Playback resumed: \
                                    WatchPage active"
                                );
                            }

                            Err(error) => {
                                eprintln!("[MutsukiTube] Resume error: {error}");
                            }
                        }
                    }
                }

                // ======================================
                // PLAY LOCAL TEST VIDEO
                // ======================================

                if !session.media_loaded {
                    if let (Some(player), Some(video_path)) =
                        (session.player.as_ref(), test_video.as_ref())
                    {
                        match player.load(video_path) {
                            Ok(()) => {
                                println!(
                                    "[MutsukiTube] Loading test video: \
                                 {video_path}"
                                );

                                session.media_loaded = true;
                            }

                            Err(error) => {
                                eprintln!(
                                    "[MutsukiTube] Playback error: \
                                 {error}"
                                );

                                // Jangan mengulangi load
                                // setiap 100 milidetik.
                                session.media_loaded = true;
                            }
                        }
                    }
                }
            },
        );

        (timer, session)
    };

    #[cfg(target_os = "windows")]
    ui.show()?;

    let result = ui.run();

    #[cfg(target_os = "windows")]
    {
        surface_timer.stop();

        // Pastikan player dilepaskan sebelum HWND.
        native_session.borrow_mut().shutdown();
    }

    result
}

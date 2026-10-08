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

#[cfg(target_os = "windows")]
mod player_controller;

#[cfg(target_os = "windows")]
use mutsukitube_player::ExternalMpvPlayer;

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

    #[cfg(target_os = "windows")]
    let native_session = Rc::new(RefCell::new(NativeVideoSession::new()));

    #[cfg(target_os = "windows")]
    handlers::player::register(&ui, &context, native_session.clone());

    #[cfg(not(target_os = "windows"))]
    handlers::player::register(&ui, &context);

    #[cfg(target_os = "windows")]
    handlers::player_controls::register(&ui, native_session.clone());

    handlers::history::register(&ui, &context);
    handlers::settings::register(&ui, &context);
    handlers::library::register(&ui, &context);
    handlers::search::register(&ui, &context);

    #[cfg(target_os = "windows")]
    let surface_timer = {
        use std::time::Duration;

        let weak = ui.as_weak();

        let session_for_timer = native_session.clone();

        let timer = slint::Timer::default();

        let last_logged_second = std::cell::Cell::new(None::<u64>);

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(100),
            move || {
                let Some(ui) = weak.upgrade() else {
                    return;
                };

                let is_watch = ui.get_current_page().as_str() == "watch";

                let mut session = session_for_timer.borrow_mut();
                session.controller.poll_events();

                let ready = session.controller.is_initialized();

                ui.set_player_ready(ready);

                if ready {
                    let position = session.controller.position().unwrap_or(0.0);

                    let duration = session.controller.duration().unwrap_or(0.0);

                    let paused = session.controller.is_paused().unwrap_or(true);

                    if position.is_finite() && position >= 0.0 {
                        ui.set_playback_position(position as f32);
                    }

                    if duration.is_finite() && duration >= 0.0 {
                        ui.set_playback_duration(duration as f32);
                    }

                    ui.set_player_paused(paused);
                }

                let position = session.controller.position();
                let duration = session.controller.duration();

                if let (Some(position), Some(duration)) = (position, duration) {
                    if position.is_finite()
                        && duration.is_finite()
                        && position >= 0.0
                        && duration > 0.0
                    {
                        let current_second = position.floor() as u64;

                        if current_second % 10 == 0
                            && last_logged_second.get() != Some(current_second)
                        {
                            last_logged_second.set(Some(current_second));

                            println!("[libmpv] Position: {:.1}s / {:.1}s", position, duration,);
                        }
                    }
                }

                session.controller.on_navigation(is_watch);

                if !is_watch {
                    session.pending_video_id = None;

                    if let Some(surface) = session.surface.as_ref() {
                        surface.hide();
                    }

                    ui.set_player_paused(true);
                    ui.set_playback_position(0.0);
                    ui.set_playback_duration(0.0);

                    return;
                }

                if session.surface.is_none() {
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

                if let Some(surface) = session.surface.as_ref() {
                    if let Err(error) = surface.set_geometry(24, 100, 640, 360) {
                        eprintln!("[MutsukiTube] Geometry error: {error}");
                    }

                    surface.show();
                }

                if !session.controller.is_initialized() {
                    let hwnd = match session.surface.as_ref() {
                        Some(surface) => surface.hwnd() as usize,
                        None => return,
                    };

                    match session.controller.initialize(hwnd) {
                        Ok(()) => {
                            println!("[MutsukiTube] Embedded player ready");

                            let initial_volume = ui.get_playback_volume() as f64;

                            if let Err(error) = session.controller.set_volume(initial_volume) {
                                eprintln!("[MutsukiTube] Initial volume error: {error}");
                            }

                            ui.set_player_ready(true);
                        }

                        Err(error) => {
                            eprintln!("[MutsukiTube] Player init error: {error}");

                            ui.set_player_ready(false);

                            if let Some(video_id) = session.pending_video_id.take() {
                                if let Err(fallback_error) =
                                    ExternalMpvPlayer::play_youtube(&video_id)
                                {
                                    eprintln!("[MutsukiTube] Fallback error: {fallback_error}");
                                }
                            }

                            return;
                        }
                    }
                }

                let Some(video_id) = session.pending_video_id.take() else {
                    return;
                };

                let url = format!("https://www.youtube.com/watch?v={video_id}");

                match session.controller.load_video(&url) {
                    Ok(()) => {
                        if let Err(error) = session.controller.play() {
                            eprintln!("[MutsukiTube] Play error: {error}");
                        }

                        println!(
                            "[MutsukiTube] Embedded video requested: \
                         {video_id}"
                        );
                    }

                    Err(error) => {
                        eprintln!("[MutsukiTube] Embedded load error: {error}");

                        if let Err(fallback_error) = ExternalMpvPlayer::play_youtube(&video_id) {
                            eprintln!(
                                "[MutsukiTube] Fallback error: \
                             {fallback_error}"
                            );
                        }
                    }
                }
            },
        );

        timer
    };

    #[cfg(target_os = "windows")]
    ui.show()?;

    let result = ui.run();

    #[cfg(target_os = "windows")]
    {
        surface_timer.stop();

        // Hentikan controller dan bebaskan HWND.
        native_session.borrow_mut().shutdown();
    }

    result
}

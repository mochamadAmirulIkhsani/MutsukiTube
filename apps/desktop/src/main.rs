mod app_context;
mod handlers;
mod native_window;
mod navigation;
mod state;
mod thumbnail;
mod thumbnail_ui;
mod ui_models;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use mutsukitube_anime::{AnimeManager, ProviderRegistry};
use mutsukitube_plugin_updater::plugin_base_dir;
use mutsukitube_storage::{get_playlist_names, get_provider_mode};
use slint::{ComponentHandle, ModelRc, VecModel};

use app_context::AppContext;
use state::AppState;

#[cfg(target_os = "windows")]
use std::cell::{Cell, RefCell};

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(target_os = "windows")]
use native_window::{
    geometry::VideoGeometry, session::NativeVideoSession, windows::Win32VideoSurface,
};

#[cfg(target_os = "windows")]
use mutsukitube_player::ExternalMpvPlayer;

#[cfg(target_os = "windows")]
mod player_controller;

slint::include_modules!();

#[cfg(test)]
mod issue_simulations;

/// Persist the currently playing anime episode's position under its own
/// identity. Called before the player switches media (episode change, leaving
/// the Watch page, or starting YouTube) so the outgoing episode keeps its own
/// progress instead of having it overwritten by the incoming media.
#[cfg(target_os = "windows")]
fn flush_anime_progress(state: &Arc<Mutex<AppState>>, position: f64, duration: f64) {
    let position_secs = position.max(0.0) as u64;

    // Nothing meaningful watched yet: do not create an empty history entry.
    if position_secs == 0 {
        return;
    }

    let duration_secs = duration.max(0.0) as u64;

    let Ok(mut app_state) = state.lock() else {
        return;
    };

    let Some(playing) = app_state.anime_now_playing.clone() else {
        return;
    };

    let key = mutsukitube_storage::anime_key(&playing.provider_id, &playing.anime_url);

    app_state
        .anime_resume
        .insert(key, (playing.episode_number, position_secs));

    let entry = mutsukitube_storage::AnimeEntry {
        provider_id: playing.provider_id.clone(),
        anime_url: playing.anime_url.clone(),
        title: playing.title.clone(),
        thumbnail_url: playing.thumbnail_url.clone(),
    };

    if let Err(error) = mutsukitube_storage::save_anime_progress(
        &entry,
        playing.episode_number,
        &playing.episode_title,
        position_secs,
        duration_secs,
    ) {
        eprintln!("[MutsukiTube] Final anime progress save failed: {error}");
    }
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
        nav_stack: vec![navigation::NavEntry::new(navigation::NavRoute::YouTubeHome)],
        ..AppState::default()
    }));

    if std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR").is_err() {
        let installed_dir = plugin_base_dir();

        if installed_dir.exists() {
            if let Some(version) = mutsukitube_plugin_updater::active_version(&installed_dir) {
                let version_dir = installed_dir.join(&version);
                let registry = ProviderRegistry::with_standard(Some(version_dir.clone()));
                let manager = AnimeManager::new(registry);
                let mut failed = false;

                if let Some(provider) = manager.provider_ids().first().cloned() {
                    if runtime.block_on(manager.info(&provider)).is_err() {
                        failed = true;
                    }
                } else {
                    failed = true;
                }

                if failed {
                    let _ =
                        mutsukitube_plugin_updater::mark_version_failed(&installed_dir, &version);
                    let _ = mutsukitube_plugin_updater::rollback_to_previous(&installed_dir);
                }
            }

            let active_dir = mutsukitube_plugin_updater::active_version(&installed_dir)
                .map(|version| installed_dir.join(version))
                .filter(|path| path.exists())
                .unwrap_or(installed_dir);

            unsafe {
                std::env::set_var("MUTSUKITUBE_ANIME_PLUGIN_DIR", &active_dir);
            }
        }
    }

    let anime_registry = ProviderRegistry::from_env();

    let anime_manager = Arc::new(AnimeManager::new(anime_registry));

    let context = AppContext {
        state: state.clone(),
        handle: tokio_handle.clone(),
        anime_manager: anime_manager.clone(),
    };

    ui.set_provider_mode(saved_provider.into());
    ui.set_current_page("youtube-home".into());

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

    #[cfg(target_os = "windows")]
    handlers::player_controls::register(&ui, native_session.clone());

    handlers::history::register(&ui, &context);
    handlers::settings::register(&ui, &context);
    handlers::library::register(&ui, &context);
    handlers::search::register(&ui, &context);
    handlers::home::register(&ui, &context);

    #[cfg(target_os = "windows")]
    handlers::anime::register(&ui, &context, native_session.clone());

    #[cfg(not(target_os = "windows"))]
    handlers::anime::register(&ui, &context);

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_back(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let _ = navigation::navigate_back(&ui, &context);
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_search(move |query| {
            let query = query.trim().to_string();

            if query.is_empty() {
                return;
            }

            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeSearch {
                    query: query.clone(),
                }),
            );

            let _ = ui.invoke_search(query.into());
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_history(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeHistory),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_favorites(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeFavorites),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_library_playlist(move |name| {
            let name = name.to_string();

            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubePlaylist { name }),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_settings(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::Settings),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_anime(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::AnimeHome),
            );

            let _ = ui.invoke_show_anime();
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_anime_details(move |key| {
            let key = key.to_string();

            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::AnimeDetails { key: key.clone() }),
            );

            let _ = ui.invoke_anime_select(key.into());
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_nav_home(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeHome),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_load_home(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeHome),
            );
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_load_home_topic(move |topic| {
            let topic = topic.to_string();

            let Some(ui) = weak.upgrade() else {
                return;
            };

            navigation::push_navigation(
                &ui,
                &context,
                navigation::NavEntry::new(navigation::NavRoute::YouTubeSearch {
                    query: topic.clone(),
                }),
            );

            let _ = ui.invoke_search(topic.into());
        });
    }

    // Kick off the YouTube Home feed on startup.
    let _ = ui.invoke_load_home();

    #[cfg(target_os = "windows")]
    {
        ui.on_open_video_browser(move |video_id| {
            let id = video_id.to_string();

            let valid = id.len() == 11
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');

            if !valid {
                if !id.is_empty() {
                    eprintln!("[MutsukiTube] Invalid browser video ID");
                }
                return;
            }

            let url = format!("https://www.youtube.com/watch?v={id}");

            match std::process::Command::new("explorer.exe").arg(&url).spawn() {
                Ok(_) => {
                    println!("[MutsukiTube] Opening video in browser");
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Browser launch error: {error}");
                }
            }
        });
    }

    {
        let weak = ui.as_weak();

        ui.on_toggle_fullscreen(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let fullscreen = !ui.window().is_fullscreen();

            ui.window().set_fullscreen(fullscreen);
            ui.set_fullscreen_mode(fullscreen);

            println!(
                "[MutsukiTube] Fullscreen: {}",
                if fullscreen { "enabled" } else { "disabled" }
            );
        });
    }

    #[cfg(target_os = "windows")]
    let surface_timer = {
        use std::time::Duration;

        let weak = ui.as_weak();
        let session_for_timer = native_session.clone();

        let timer = slint::Timer::default();

        let last_geometry = Cell::new(None::<VideoGeometry>);
        let surface_visible = Cell::new(false);
        let was_minimized = Cell::new(false);
        let video_loading = Cell::new(false);
        let last_playback_state = RefCell::new(String::new());
        let seeking_ticks = Cell::new(0u32);
        let state_for_timer = state.clone();
        let progress_handle = tokio_handle.clone();

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(200),
            move || {
                let Some(ui) = weak.upgrade() else {
                    return;
                };

                let is_watch = ui.get_current_page().as_str() == "watch";

                let is_minimized = ui.window().is_minimized();

                if was_minimized.get() != is_minimized {
                    was_minimized.set(is_minimized);

                    if is_minimized {
                        println!("[MutsukiTube] Window minimized");
                    } else {
                        println!("[MutsukiTube] Window restored");

                        last_geometry.set(None);
                    }
                }

                let mut session = session_for_timer.borrow_mut();

                session.controller.poll_events();

                let playback_state = session.controller.playback_state().as_str().to_string();

                {
                    let mut previous = last_playback_state.borrow_mut();

                    if *previous != playback_state {
                        println!(
                            "[MutsukiTube] Playback state: {} -> {}",
                            if previous.is_empty() {
                                "unknown"
                            } else {
                                previous.as_str()
                            },
                            playback_state
                        );

                        *previous = playback_state.clone();
                    }
                }

                if session.pending_video_id.is_some() {
                    video_loading.set(true);
                } else if video_loading.get() {
                    match playback_state.as_str() {
                        "playing" | "paused" | "finished" | "stopped" | "error" => {
                            video_loading.set(false);
                        }
                        _ => {}
                    }
                }

                if video_loading.get() {
                    ui.set_playback_state("loading".into());
                } else {
                    ui.set_playback_state(playback_state.clone().into());
                }

                let ready = session.controller.is_initialized();

                ui.set_player_ready(ready);

                if ready {
                    // Only the currently loaded media's own timeline reaches the
                    // UI. `timeline()` returns empty values while the next file
                    // loads, so a previous episode's duration/position cannot
                    // leak onto the incoming episode's slider.
                    let timeline = session.controller.timeline();

                    let position = timeline.position;

                    let duration = timeline.duration;

                    let paused = session.controller.is_paused().unwrap_or(true);

                    ui.set_player_paused(paused);

                    if timeline.has_duration {
                        ui.set_playback_duration(duration as f32);
                    } else {
                        // Unknown duration for the current media: clear any stale
                        // maximum so the slider is disabled and no old episode's
                        // range lingers.
                        ui.set_playback_duration(0.0);
                    }

                    // Never yank the thumb while the user is dragging the slider (auto-clears after 1.6s).
                    if ui.get_timeline_seek_active() {
                        let ticks = seeking_ticks.get() + 1;
                        if ticks > 8 {
                            ui.set_timeline_seek_active(false);
                            seeking_ticks.set(0);
                            ui.set_playback_position(position as f32);
                        } else {
                            seeking_ticks.set(ticks);
                        }
                    } else {
                        seeking_ticks.set(0);


                        ui.set_playback_position(position as f32);
                    }

                    // A media switch is queued but not yet applied: the player is
                    // still on the outgoing episode, so neither resume nor
                    // progress may be attributed to it.
                    let transition_pending = session.pending_anime_source.is_some()
                        || session.pending_video_id.is_some();

                    if is_watch
                        && playback_state == "playing"
                        && session.controller.media_loaded()
                        && !transition_pending
                    {
                        // Apply a pending resume seek once playback is running.
                        if let Some(resume) = session.pending_anime_resume {
                            if resume > 1.0 && position + 2.0 < resume {
                                if let Err(error) = session.controller.seek(resume) {
                                    eprintln!("[MutsukiTube] Resume seek error: {error}");
                                } else {
                                    println!("[MutsukiTube] Resumed anime at {resume:.0}s");
                                }
                            }

                            session.pending_anime_resume = None;
                        }

                        // Persist playback progress at most every 5 seconds.
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        let pending = match state_for_timer.lock() {
                            Ok(mut st) => {
                                if now.saturating_sub(st.anime_last_progress_save) < 5 {
                                    None
                                } else {
                                    st.anime_last_progress_save = now;

                                    st.anime_now_playing.clone().map(|playing| {
                                        let key = mutsukitube_storage::anime_key(
                                            &playing.provider_id,
                                            &playing.anime_url,
                                        );

                                        st.anime_resume.insert(
                                            key,
                                            (playing.episode_number, position.max(0.0) as u64),
                                        );

                                        playing
                                    })
                                }
                            }
                            Err(_) => None,
                        };

                        if let Some(playing) = pending {
                            let position_secs = position.max(0.0) as u64;
                            let duration_secs = duration.max(0.0) as u64;

                            let handle = progress_handle.clone();

                            handle.spawn(async move {
                                let entry = mutsukitube_storage::AnimeEntry {
                                    provider_id: playing.provider_id.clone(),
                                    anime_url: playing.anime_url.clone(),
                                    title: playing.title.clone(),
                                    thumbnail_url: playing.thumbnail_url.clone(),
                                };

                                if let Err(error) = mutsukitube_storage::save_anime_progress(
                                    &entry,
                                    playing.episode_number,
                                    &playing.episode_title,
                                    position_secs,
                                    duration_secs,
                                ) {
                                    eprintln!("[MutsukiTube] Progress save failed: {error}");
                                }
                            });
                        }
                    }
                }

                // Leaving the Watch page: give the outgoing episode its final
                // progress write, then forget the durable identity so nothing
                // else can be attributed to it. `anime_pending_playing` is left
                // alone: a just-requested episode may still be resolving its
                // stream and will consume it when the media actually loads.
                if ready && !is_watch {
                    let position = session.controller.position().unwrap_or(0.0);
                    let duration = session.controller.duration().unwrap_or(0.0);

                    flush_anime_progress(&state_for_timer, position, duration);

                    if let Ok(mut st) = state_for_timer.lock() {
                        st.anime_now_playing = None;
                    }
                }

                session.controller.on_navigation(is_watch);

                if !is_watch {
                    session.pending_video_id = None;
                    video_loading.set(false);

                    if surface_visible.get() {
                        if let Some(surface) = session.surface.as_ref() {
                            surface.hide();
                        }

                        surface_visible.set(false);

                        println!("[MutsukiTube] Video surface hidden");
                    }

                    ui.set_player_paused(true);
                    ui.set_playback_position(0.0);
                    ui.set_playback_duration(0.0);

                    last_geometry.set(None);

                    return;
                }

                if is_minimized {
                    if surface_visible.get() {
                        if let Some(surface) = session.surface.as_ref() {
                            surface.hide();
                        }

                        surface_visible.set(false);

                        println!("[MutsukiTube] Video surface hidden");
                    }

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

                let geometry = VideoGeometry::from_slint(&ui);

                if ui.get_fullscreen_mode() && geometry.is_none() {
                    eprintln!(
                        "[MutsukiTube] Fullscreen geometry invalid: \
                         x={}, y={}, width={}, height={}",
                        ui.get_video_x(),
                        ui.get_video_y(),
                        ui.get_video_width(),
                        ui.get_video_height(),
                    );
                }

                if let Some(surface) = session.surface.as_ref() {
                    match geometry {
                        Some(rect) => {
                            if last_geometry.get() != Some(rect) {
                                match surface.set_geometry(rect.x, rect.y, rect.width, rect.height)
                                {
                                    Ok(()) => {
                                        last_geometry.set(Some(rect));

                                        println!(
                                            "[MutsukiTube] Video surface: \
                                             {}x{} at ({}, {})",
                                            rect.width, rect.height, rect.x, rect.y,
                                        );
                                    }
                                    Err(error) => {
                                        eprintln!("[MutsukiTube] Geometry error: {error}");

                                        return;
                                    }
                                }
                            }
                        }
                        None => {
                            if surface_visible.get() {
                                surface.hide();
                                surface_visible.set(false);

                                println!("[MutsukiTube] Video surface hidden");
                            }

                            last_geometry.set(None);

                            return;
                        }
                    }
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
                            ui.set_playback_state("error".into());

                            video_loading.set(false);

                            if let Some(video_id) = session.pending_video_id.take() {
                                if let Err(fallback_error) =
                                    ExternalMpvPlayer::play_youtube(&video_id)
                                {
                                    eprintln!(
                                        "[MutsukiTube] Fallback error: \
                                         {fallback_error}"
                                    );
                                }
                            }

                            return;
                        }
                    }
                }

                if let Some(source) = session.pending_anime_source.take() {
                    let headers = std::mem::take(&mut session.pending_anime_headers);

                    // Persist the outgoing episode (if any) before the new media
                    // replaces it, then promote the resolved episode to the
                    // durable "now playing" identity.
                    flush_anime_progress(
                        &state_for_timer,
                        session.controller.position().unwrap_or(0.0),
                        session.controller.duration().unwrap_or(0.0),
                    );

                    if let Ok(mut st) = state_for_timer.lock() {
                        st.anime_now_playing = st.anime_pending_playing.take();
                    }

                    video_loading.set(true);

                    ui.set_playback_state("loading".into());
                    ui.set_playback_position(0.0);
                    ui.set_playback_duration(0.0);
                    ui.set_player_paused(true);

                    match session.controller.load_media(&source, &headers) {
                        Ok(()) => {
                            if let Err(error) = session.controller.play() {
                                eprintln!("[MutsukiTube] Play error: {error}");
                            }

                            println!("[MutsukiTube] Embedded anime source requested");

                            println!(
                                "[MutsukiTube] Anime media generation {}",
                                session.controller.media_generation()
                            );
                        }
                        Err(error) => {
                            eprintln!("[MutsukiTube] Embedded anime load error: {error}");

                            video_loading.set(false);
                            ui.set_playback_state("error".into());
                        }
                    }
                } else if let Some(video_id) = session.pending_video_id.take() {
                    // Switching to YouTube: the anime episode is no longer the
                    // player's media, so stop attributing timeline/progress to
                    // it. `anime_pending_*` are intentionally left for the next
                    // resolved anime load to overwrite/consume.
                    flush_anime_progress(
                        &state_for_timer,
                        session.controller.position().unwrap_or(0.0),
                        session.controller.duration().unwrap_or(0.0),
                    );

                    if let Ok(mut st) = state_for_timer.lock() {
                        st.anime_now_playing = None;
                    }

                    video_loading.set(true);

                    ui.set_playback_state("loading".into());
                    ui.set_playback_position(0.0);
                    ui.set_playback_duration(0.0);
                    ui.set_player_paused(true);

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

                            video_loading.set(false);
                            ui.set_playback_state("error".into());

                            if let Err(fallback_error) = ExternalMpvPlayer::play_youtube(&video_id)
                            {
                                eprintln!(
                                    "[MutsukiTube] Fallback error: \
                                     {fallback_error}"
                                );
                            }
                        }
                    }
                }

                let controller_state = session.controller.playback_state().as_str();

                if video_loading.get() && controller_state == "error" {
                    video_loading.set(false);
                    ui.set_playback_state("error".into());
                }

                let is_loading = video_loading.get();

                let controller_state = session.controller.playback_state().as_str();

                let should_show_surface = is_watch
                    && !is_minimized
                    && !is_loading
                    && controller_state != "error"
                    && geometry.is_some();

                if let Some(surface) = session.surface.as_ref() {
                    if should_show_surface {
                        if !surface_visible.get() {
                            surface.show();
                            surface_visible.set(true);

                            println!("[MutsukiTube] Video surface shown");
                        }
                    } else if surface_visible.get() {
                        surface.hide();
                        surface_visible.set(false);

                        println!("[MutsukiTube] Video surface hidden (loading)");
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
        native_session.borrow_mut().shutdown();
    }

    result
}

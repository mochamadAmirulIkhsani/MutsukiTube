use slint::ComponentHandle;

use mutsukitube_core::Video;
use mutsukitube_storage::save_video;

use crate::{AppWindow, app_context::AppContext, state::AppState};

#[cfg(target_os = "windows")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_os = "windows")]
use crate::native_window::session::NativeVideoSession;

fn find_video(state: &AppState, video_id: &str) -> Option<Video> {
    state
        .search_results
        .iter()
        .chain(state.history_results.iter())
        .chain(state.library_results.iter())
        .find(|video| video.id == video_id)
        .cloned()
}

fn is_valid_video_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

#[cfg(target_os = "windows")]
pub fn register(ui: &AppWindow, context: &AppContext, session: Rc<RefCell<NativeVideoSession>>) {
    register_play_video(ui, session.clone());
    register_open_video(ui, context, session);
}

#[cfg(not(target_os = "windows"))]
pub fn register(ui: &AppWindow, context: &AppContext) {
    register_play_video(ui);
    register_open_video(ui, context);
}

#[cfg(target_os = "windows")]
fn register_open_video(
    ui: &AppWindow,
    context: &AppContext,
    session: Rc<RefCell<NativeVideoSession>>,
) {
    let weak = ui.as_weak();
    let state = context.state.clone();
    let handle = context.handle.clone();

    ui.on_open_video(move |video_id| {
        let video_id = video_id.to_string();

        if !is_valid_video_id(&video_id) {
            eprintln!("[MutsukiTube] Invalid YouTube video ID: {video_id}");
            return;
        }

        let video = state
            .lock()
            .ok()
            .and_then(|state| find_video(&state, &video_id));

        let Some(video) = video else {
            eprintln!("[MutsukiTube] Video not found: {video_id}");
            return;
        };

        let Some(ui) = weak.upgrade() else {
            return;
        };

        ui.set_selected_video_id(video.id.clone().into());
        ui.set_selected_video_title(video.title.clone().into());
        ui.set_selected_video_channel(video.channel.clone().into());

        ui.set_playback_position(0.0);
        ui.set_playback_duration(0.0);
        ui.set_player_paused(true);
        ui.set_playback_state("loading".into());

        session.borrow_mut().request_video(video.id.clone());

        ui.set_current_page("watch".into());

        println!("[MutsukiTube] Autoplay queued: {}", video.id);

        handle.spawn_blocking(move || {
            if let Err(error) = save_video(&video) {
                eprintln!("[MutsukiTube] History error: {error}");
            }
        });
    });
}

#[cfg(not(target_os = "windows"))]
fn register_open_video(ui: &AppWindow, context: &AppContext) {
    use mutsukitube_player::ExternalMpvPlayer;

    let weak = ui.as_weak();
    let state = context.state.clone();
    let handle = context.handle.clone();

    ui.on_open_video(move |video_id| {
        let video_id = video_id.to_string();

        if !is_valid_video_id(&video_id) {
            eprintln!("[MutsukiTube] Invalid YouTube video ID: {video_id}");
            return;
        }

        let video = state
            .lock()
            .ok()
            .and_then(|state| find_video(&state, &video_id));

        let Some(video) = video else {
            eprintln!("[MutsukiTube] Video not found: {video_id}");
            return;
        };

        let Some(ui) = weak.upgrade() else {
            return;
        };

        ui.set_selected_video_id(video.id.clone().into());
        ui.set_selected_video_title(video.title.clone().into());
        ui.set_selected_video_channel(video.channel.clone().into());

        ui.set_playback_position(0.0);
        ui.set_playback_duration(0.0);
        ui.set_player_paused(true);
        ui.set_playback_state("loading".into());

        ui.set_current_page("watch".into());

        let playback_id = video.id.clone();

        handle.spawn_blocking(move || {
            if let Err(error) = save_video(&video) {
                eprintln!("[MutsukiTube] History error: {error}");
            }

            if let Err(error) = ExternalMpvPlayer::play_youtube(&playback_id) {
                eprintln!("[MutsukiTube] External player error: {error}");
            }
        });
    });
}

#[cfg(target_os = "windows")]
fn register_play_video(ui: &AppWindow, session: Rc<RefCell<NativeVideoSession>>) {
    ui.on_play_video(move |video_id| {
        let id = video_id.to_string();

        if !is_valid_video_id(&id) {
            eprintln!("[MutsukiTube] Invalid YouTube video ID: {id}");
            return;
        }

        session.borrow_mut().request_video(id);

        println!("[MutsukiTube] Embedded playback requested");
    });
}

#[cfg(not(target_os = "windows"))]
fn register_play_video(ui: &AppWindow) {
    use mutsukitube_player::ExternalMpvPlayer;

    ui.on_play_video(move |video_id| {
        let id = video_id.to_string();

        if !is_valid_video_id(&id) {
            eprintln!("[MutsukiTube] Invalid YouTube video ID: {id}");
            return;
        }

        if let Err(error) = ExternalMpvPlayer::play_youtube(&id) {
            eprintln!("[MutsukiTube] External player error: {error}");
        }
    });
}

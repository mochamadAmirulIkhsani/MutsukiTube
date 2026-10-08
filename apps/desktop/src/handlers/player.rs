use slint::ComponentHandle;

use mutsukitube_core::Video;
use mutsukitube_player::ExternalMpvPlayer;
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

pub fn register(
    ui: &AppWindow,
    context: &AppContext,
    #[cfg(target_os = "windows")] session: Rc<RefCell<NativeVideoSession>>,
) {
    register_open_video(ui, context);

    #[cfg(target_os = "windows")]
    register_play_video(ui, session);

    #[cfg(not(target_os = "windows"))]
    register_play_video(ui);
}

fn register_open_video(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let state = context.state.clone();
    let handle = context.handle.clone();

    ui.on_open_video(move |video_id| {
        let video_id = video_id.to_string();

        let video = state.lock().ok().and_then(|s| find_video(&s, &video_id));

        let Some(video) = video else {
            eprintln!("[MutsukiTube] Video not found: {video_id}");
            return;
        };

        if let Some(ui) = weak.upgrade() {
            ui.set_selected_video_id(video.id.clone().into());

            ui.set_selected_video_title(video.title.clone().into());

            ui.set_selected_video_channel(video.channel.clone().into());

            ui.set_watch_library_status("".into());

            ui.set_current_page("watch".into());
        }

        // SQLite berjalan di blocking thread.
        handle.spawn_blocking(move || {
            if let Err(error) = save_video(&video) {
                eprintln!("[MutsukiTube] History error: {error}");
            }
        });
    });
}

#[cfg(target_os = "windows")]
fn register_play_video(ui: &AppWindow, session: Rc<RefCell<NativeVideoSession>>) {
    ui.on_play_video(move |video_id| {
        let id = video_id.to_string();

        let valid = id.len() == 11
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');

        if !valid {
            eprintln!("[MutsukiTube] Invalid YouTube video ID");
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
        if let Err(error) = ExternalMpvPlayer::play_youtube(video_id.as_str()) {
            eprintln!("[MutsukiTube] External player error: {error}");
        }
    });
}

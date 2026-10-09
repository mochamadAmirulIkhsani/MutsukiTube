use std::{cell::RefCell, rc::Rc};

use slint::ComponentHandle;

use crate::{AppWindow, native_window::session::NativeVideoSession};

pub fn register(ui: &AppWindow, session: Rc<RefCell<NativeVideoSession>>) {
    // ===================================
    // PLAY / PAUSE
    // ===================================
    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_toggle_playback(move || {
            let result = session.borrow().controller.toggle_pause();

            match result {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        let paused = session.borrow().controller.is_paused().unwrap_or(true);

                        ui.set_player_paused(paused);
                    }
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Toggle error: {error}");
                }
            }
        });
    }

    // ===================================
    // STOP
    // ===================================
    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_stop_playback(move || {
            let result = session.borrow_mut().controller.stop();

            match result {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_player_paused(true);
                        ui.set_playback_position(0.0);
                    }
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Stop error: {error}");
                }
            }
        });
    }

    // ===================================
    // SEEK
    // ===================================
    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_seek_playback(move |seconds| {
            let seconds = seconds as f64;

            if let Err(error) = session.borrow().controller.seek(seconds) {
                eprintln!("[MutsukiTube] Seek error: {error}");
                return;
            }

            if let Some(ui) = weak.upgrade() {
                ui.set_playback_position(seconds as f32);
            }
        });
    }

    // ===================================
    // VOLUME
    // ===================================
    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_change_volume(move |value| {
            let volume = (value as f64).clamp(0.0, 100.0);

            if let Err(error) = session.borrow().controller.set_volume(volume) {
                eprintln!("[MutsukiTube] Volume error: {error}");
                return;
            }

            if let Some(ui) = weak.upgrade() {
                ui.set_playback_volume(volume as f32);
            }
        });
    }

    // ===================================
    // REPLAY
    // ===================================
    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_replay_video(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            if ui.get_playback_state().as_str() != "finished" {
                return;
            }

            let result = session.borrow_mut().controller.replay();

            match result {
                Ok(()) => {
                    ui.set_playback_state("loading".into());
                    ui.set_playback_position(0.0);

                    println!("[MutsukiTube] Replay requested");
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Replay failed: {error}");

                    ui.set_playback_state("error".into());
                }
            }
        });
    }
}

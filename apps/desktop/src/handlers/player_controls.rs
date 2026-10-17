use std::{cell::Cell, cell::RefCell, rc::Rc};

use slint::ComponentHandle;

use crate::{AppWindow, native_window::session::NativeVideoSession};

pub fn register(ui: &AppWindow, session: Rc<RefCell<NativeVideoSession>>) {
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

    {
        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_seek_playback(move |seconds| {
            let seconds = seconds as f64;

            if !seconds.is_finite() || seconds < 0.0 {
                eprintln!("[MutsukiTube] Invalid seek position: {seconds}");
                return;
            }

            let duration = session.borrow().controller.duration().unwrap_or(0.0);

            if !duration.is_finite() || duration <= 0.0 {
                eprintln!("[MutsukiTube] Cannot seek: invalid duration");
                return;
            }

            let target = seconds.clamp(0.0, duration);

            println!("[MutsukiTube] Seek requested: {:.2}s", target);

            let result = session.borrow().controller.seek(target);

            match result {
                Ok(()) => {
                    if let Some(ui) = weak.upgrade() {
                        ui.set_playback_position(target as f32);
                    }

                    println!("[MutsukiTube] Seek command accepted: {:.2}s", target);
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Seek error: {error}");
                }
            }
        });
    }

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
                ui.set_player_muted(volume <= 0.0);

                println!("[MutsukiTube] Volume changed: {:.0}%", volume);
            }
        });
    }

    {
        let previous_volume = Rc::new(Cell::new(70.0_f64));

        let session = session.clone();
        let weak = ui.as_weak();

        ui.on_toggle_mute(move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };

            let currently_muted = ui.get_player_muted();

            let current_volume = ui.get_playback_volume() as f64;

            let target_volume = if currently_muted {
                previous_volume.get()
            } else {
                if current_volume > 0.0 {
                    previous_volume.set(current_volume);
                }

                0.0
            };

            let result = session.borrow().controller.set_volume(target_volume);

            match result {
                Ok(()) => {
                    ui.set_playback_volume(target_volume as f32);

                    ui.set_player_muted(target_volume <= 0.0);

                    println!("[MutsukiTube] Volume: {}%", target_volume);
                }

                Err(error) => {
                    eprintln!("[MutsukiTube] Mute error: {error}");
                }
            }
        });
    }

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

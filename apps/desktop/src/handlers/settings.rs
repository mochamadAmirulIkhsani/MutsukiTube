use mutsukitube_storage::{set_provider_mode, validate_provider_mode};

use slint::ComponentHandle;

use crate::{AppWindow, app_context::AppContext};

pub fn register(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_save_provider(move |mode| {
        let mode = mode.to_string();

        if !validate_provider_mode(&mode) {
            if let Some(ui) = weak.upgrade() {
                ui.set_settings_status("Invalid provider mode".into());
            }
            return;
        }

        if let Some(ui) = weak.upgrade() {
            ui.set_settings_status("Saving settings...".into());
        }

        let weak = weak.clone();
        let state = context.state.clone();

        context.handle.spawn_blocking(move || {
            let result = set_provider_mode(&mode);

            let _ = weak.upgrade_in_event_loop(move |ui| match result {
                Ok(()) => {
                    if let Ok(mut s) = state.lock() {
                        s.provider_mode = mode.clone();

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
                    ui.set_settings_status(format!("Failed to save settings: {error}").into());
                }
            });
        });
    });
}

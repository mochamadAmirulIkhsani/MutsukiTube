use mutsukitube_storage::{
    clear_subscriptions, get_subscriptions_count, import_subscriptions, parse_subscriptions_csv,
    set_provider_mode, validate_provider_mode,
};

use slint::ComponentHandle;

use crate::{AppWindow, app_context::AppContext};

#[cfg(target_os = "windows")]
fn pick_csv_file() -> Option<String> {
    let script = r#"
[System.Reflection.Assembly]::LoadWithPartialName("System.Windows.Forms") | Out-Null
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Filter = "CSV Files (*.csv)|*.csv|All Files (*.*)|*.*"
$dialog.Title = "Select YouTube Subscriptions CSV"
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Write($dialog.FileName)
}
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn pick_csv_file() -> Option<String> {
    None
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    if let Ok(count) = get_subscriptions_count() {
        ui.set_subscriptions_count(count as i32);
    }

    register_provider_mode(ui, context);
    register_importer(ui, context);
}

fn register_provider_mode(ui: &AppWindow, context: &AppContext) {
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

fn register_importer(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let handle = context.handle.clone();

    ui.on_select_import_file({
        let weak = weak.clone();
        let handle = handle.clone();
        move || {
            let weak = weak.clone();
            handle.spawn_blocking(move || {
                if let Some(path) = pick_csv_file() {
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_import_file_path(path.into());
                        ui.set_import_status("".into());
                        ui.set_import_is_error(false);
                    });
                }
            });
        }
    });

    ui.on_import_subscriptions({
        let weak = weak.clone();
        let handle = handle.clone();
        move |raw_path| {
            let path = raw_path.to_string().trim().trim_matches('"').to_string();
            if path.is_empty() {
                if let Some(ui) = weak.upgrade() {
                    ui.set_import_status("Please select or enter a CSV file path".into());
                    ui.set_import_is_error(true);
                }
                return;
            }

            if let Some(ui) = weak.upgrade() {
                ui.set_import_loading(true);
                ui.set_import_status("Reading CSV file...".into());
                ui.set_import_is_error(false);
            }

            let weak = weak.clone();
            handle.spawn_blocking(move || {
                let content = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(err) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_import_loading(false);
                            ui.set_import_is_error(true);
                            ui.set_import_status(format!("Failed to read file: {err}").into());
                        });
                        return;
                    }
                };

                let channels = parse_subscriptions_csv(&content);
                if channels.is_empty() {
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_import_loading(false);
                        ui.set_import_is_error(true);
                        ui.set_import_status(
                            "No valid channel IDs found. Ensure CSV is from Google Takeout (with UC... channel IDs)."
                                .into(),
                        );
                    });
                    return;
                }

                let total_imported = channels.len();
                match import_subscriptions(&channels) {
                    Ok(saved) => {
                        let count = get_subscriptions_count().unwrap_or(saved);
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_import_loading(false);
                            ui.set_import_is_error(false);
                            ui.set_subscriptions_count(count as i32);
                            ui.set_import_status(
                                format!(
                                    "Successfully imported {} subscribed channels!",
                                    total_imported
                                )
                                .into(),
                            );
                        });
                    }
                    Err(err) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_import_loading(false);
                            ui.set_import_is_error(true);
                            ui.set_import_status(
                                format!("Failed to save subscriptions: {err}").into(),
                            );
                        });
                    }
                }
            });
        }
    });

    ui.on_clear_subscriptions({
        let weak = weak.clone();
        let handle = handle.clone();
        move || {
            let weak = weak.clone();
            handle.spawn_blocking(move || {
                let result = clear_subscriptions();
                let _ = weak.upgrade_in_event_loop(move |ui| match result {
                    Ok(deleted) => {
                        ui.set_subscriptions_count(0);
                        ui.set_import_status(format!("Cleared {deleted} subscriptions").into());
                        ui.set_import_is_error(false);
                    }
                    Err(err) => {
                        ui.set_import_status(
                            format!("Failed to clear subscriptions: {err}").into(),
                        );
                        ui.set_import_is_error(true);
                    }
                });
            });
        }
    });
}

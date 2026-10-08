use std::rc::Rc;

use mutsukitube_storage::get_history;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::{
    AppWindow,
    app_context::AppContext,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::video_to_ui,
};

fn load_history(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.history_generation = state.history_generation.wrapping_add(1);

        state.history_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_history_loading(true);
        ui.set_history_status("Loading history...".into());
    }

    let handle = context.handle.clone();

    handle.spawn(async move {
        let result = tokio::task::spawn_blocking(|| get_history(100)).await;

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.history_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            ui.set_history_loading(false);

            match result {
                Ok(Ok(videos)) => {
                    let count = videos.len();

                    if let Ok(mut state) = context.state.lock() {
                        state.history_results = videos.clone();
                    }

                    let items = videos
                        .iter()
                        .map(|video| video_to_ui(video, slint::Image::default()))
                        .collect::<Vec<_>>();

                    ui.set_history_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                    ui.set_history_status(format!("{count} videos in history").into());

                    queue_thumbnails(
                        videos,
                        ThumbnailTarget::History,
                        generation,
                        ui.as_weak(),
                        context.state.clone(),
                        context.handle.clone(),
                    );
                }

                Ok(Err(error)) => {
                    ui.set_history_status(format!("History error: {error}").into());
                }

                Err(error) => {
                    ui.set_history_status(format!("Task error: {error}").into());
                }
            }
        });
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_show_history(move || {
            load_history(weak.clone(), context.clone());
        });
    }

    {
        let weak = ui.as_weak();
        let context = context.clone();

        ui.on_refresh_history(move || {
            load_history(weak.clone(), context.clone());
        });
    }
}

mod state;
mod thumbnail;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use mutsukitube_core::{Video, VideoProvider};

use mutsukitube_player::ExternalMpvPlayer;

use mutsukitube_youtube::YoutubeProvider;

use slint::{ComponentHandle, ModelRc, VecModel};

use state::AppState;

use thumbnail::download_thumbnail;

use futures::future::join_all;

slint::include_modules!();

fn format_duration(seconds: Option<u64>) -> String {
    let Some(seconds) = seconds else {
        return String::new();
    };

    let hours = seconds / 3600;

    let minutes = (seconds % 3600) / 60;

    let seconds = seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

fn video_to_ui(video: &Video, thumbnail: slint::Image) -> VideoItem {
    VideoItem {
        id: video.id.clone().into(),

        title: video.title.clone().into(),

        channel: video.channel.clone().into(),

        duration: format_duration(video.duration).into(),

        thumbnail,
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    let runtime = Rc::new(runtime);

    let state = Arc::new(Mutex::new(AppState::default()));

    {
        let ui_weak = ui.as_weak();

        let runtime = runtime.clone();

        let state = state.clone();

        ui.on_search(move |query| {
            let query = query.to_string();

            if query.trim().is_empty() {
                return;
            }

            if let Some(ui) = ui_weak.upgrade() {
                ui.set_loading(true);

                ui.set_status_text("".into());
            }

            let ui_weak = ui_weak.clone();

            let state = state.clone();

            runtime.spawn(async move {
                let provider = YoutubeProvider::new();

                let result = provider.search(&query).await;

                match result {
                    Ok(videos) => {
                        if let Ok(mut state) = state.lock() {
                            state.search_results = videos.clone();
                        }

                        let tasks = videos.into_iter().map(|video| async move {
                            let bytes = download_thumbnail(&video.thumbnail).await;

                            (video, bytes)
                        });

                        let prepared = join_all(tasks).await;

                        let count = prepared.len();

                        ui_weak
                            .upgrade_in_event_loop(move |ui| {
                                let items = prepared
                                    .into_iter()
                                    .map(|(video, bytes)| {
                                        let image = bytes
                                            .and_then(|bytes| {
                                                slint::Image::load_from_data(&bytes, None).ok()
                                            })
                                            .unwrap_or_default();

                                        video_to_ui(&video, image)
                                    })
                                    .collect::<Vec<_>>();

                                let model = Rc::new(VecModel::from(items));

                                ui.set_videos(ModelRc::from(model));

                                ui.set_loading(false);

                                ui.set_status_text(format!("{count} videos found").into());
                            })
                            .ok();
                    }

                    Err(error) => {
                        ui_weak
                            .upgrade_in_event_loop(move |ui| {
                                ui.set_loading(false);

                                ui.set_status_text(format!("Search failed: {error}").into());
                            })
                            .ok();
                    }
                }
            });
        });
    }

    //
    // OPEN VIDEO
    //

    {
        let ui_weak = ui.as_weak();

        let state = state.clone();

        ui.on_open_video(move |video_id| {
            let video_id = video_id.to_string();

            let video = {
                let Ok(state) = state.lock() else {
                    return;
                };

                state
                    .search_results
                    .iter()
                    .find(|video| video.id == video_id)
                    .cloned()
            };

            let Some(video) = video else {
                return;
            };

            if let Some(ui) = ui_weak.upgrade() {
                ui.set_selected_video_id(video.id.into());

                ui.set_selected_video_title(video.title.into());

                ui.set_selected_video_channel(video.channel.into());

                ui.set_current_page("watch".into());
            }
        });
    }
    //
    // PLAY VIDEO
    //

    ui.on_play_video(move |video_id| {
        if let Err(error) = ExternalMpvPlayer::play_youtube(video_id.as_str()) {
            eprintln!("player error: {error}");
        }
    });

    ui.run()
}

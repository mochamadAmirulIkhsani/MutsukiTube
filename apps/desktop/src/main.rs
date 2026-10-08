mod state;
mod thumbnail;

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use futures::future::join_all;

use mutsukitube_core::{ProviderError, Video, VideoProvider};

use mutsukitube_player::ExternalMpvPlayer;

use mutsukitube_youtube::{NativeYoutubeProvider, ProviderMode, SearchPage, YtDlpProvider};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use state::AppState;
use thumbnail::download_thumbnail;

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

// Provider untuk halaman pertama.
// Mode auto mencoba native lalu fallback.
async fn fetch_first_page(query: &str) -> Result<SearchPage, ProviderError> {
    let mode = ProviderMode::from_env();

    match mode {
        ProviderMode::Native => NativeYoutubeProvider::new().search_page(query, None).await,

        ProviderMode::YtDlp => {
            let videos = YtDlpProvider::new().search(query).await?;

            Ok(SearchPage {
                videos,
                next_page_token: None,
            })
        }

        ProviderMode::Auto => match NativeYoutubeProvider::new().search_page(query, None).await {
            Ok(page) if !page.videos.is_empty() => {
                println!("[MutsukiTube] Native: {} results", page.videos.len());
                Ok(page)
            }

            Ok(_) | Err(_) => {
                eprintln!("[MutsukiTube] Falling back to yt-dlp");

                let videos = YtDlpProvider::new().search(query).await?;

                Ok(SearchPage {
                    videos,
                    next_page_token: None,
                })
            }
        },
    }
}

fn queue_thumbnails(
    videos: Vec<Video>,
    generation: u64,
    weak: slint::Weak<AppWindow>,
    state: Arc<Mutex<AppState>>,
    handle: tokio::runtime::Handle,
) {
    handle.spawn(async move {
        let jobs = videos.into_iter().map(|video| {
            let weak = weak.clone();
            let state = state.clone();

            async move {
                let bytes = download_thumbnail(&video.thumbnail).await;

                let Some(bytes) = bytes else {
                    return;
                };

                let video_id = video.id;

                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let current = state
                        .lock()
                        .map(|s| s.search_generation)
                        .unwrap_or_default();

                    if current != generation {
                        return;
                    }

                    let Ok(image) = slint::Image::load_from_data(&bytes, None) else {
                        return;
                    };

                    let model = ui.get_videos();

                    // Update hanya card yang sesuai ID.
                    // Semua manipulasi Slint tetap di UI thread.
                    let mut rows = (0..model.row_count())
                        .filter_map(|i| model.row_data(i))
                        .collect::<Vec<_>>();

                    let Some(item) = rows.iter_mut().find(|item| item.id.as_str() == video_id)
                    else {
                        return;
                    };

                    item.thumbnail = image;

                    ui.set_videos(ModelRc::from(Rc::new(VecModel::from(rows))));
                });
            }
        });

        join_all(jobs).await;
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let runtime = Rc::new(tokio::runtime::Runtime::new().expect("failed to create Tokio runtime"));

    let tokio_handle = runtime.handle().clone();

    let state = Arc::new(Mutex::new(AppState::default()));

    // ========================================
    // SEARCH — FIRST PAGE
    // ========================================
    {
        let weak = ui.as_weak();
        let runtime = runtime.clone();
        let state = state.clone();
        let tokio_handle = tokio_handle.clone();

        ui.on_search(move |query| {
            let query = query.trim().to_string();

            if query.is_empty() {
                return;
            }

            let generation = {
                let mut s = state.lock().unwrap();

                s.search_generation = s.search_generation.wrapping_add(1);

                s.current_query = query.clone();
                s.search_results.clear();
                s.continuation_token = None;
                s.loading_more = false;

                s.search_generation
            };

            if let Some(ui) = weak.upgrade() {
                ui.set_loading(true);
                ui.set_loading_more(false);
                ui.set_has_more(false);
                ui.set_status_text("".into());

                ui.set_videos(ModelRc::from(Rc::new(VecModel::<VideoItem>::default())));
            }

            let weak = weak.clone();
            let state = state.clone();
            let handle_for_images = tokio_handle.clone();

            runtime.spawn(async move {
                let result = fetch_first_page(&query).await;

                match result {
                    Ok(page) => {
                        let videos = page.videos.clone();
                        let next_token = page.next_page_token;
                        let count = videos.len();

                        let weak_for_images = weak.clone();
                        let state_for_images = state.clone();

                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.search_results = videos.clone();
                            s.continuation_token = next_token;

                            let has_more = s.continuation_token.is_some();

                            drop(s);

                            let items = videos
                                .iter()
                                .map(|video| video_to_ui(video, slint::Image::default()))
                                .collect::<Vec<_>>();

                            ui.set_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                            ui.set_loading(false);
                            ui.set_has_more(has_more);
                            ui.set_status_text(format!("{count} videos found").into());

                            queue_thumbnails(
                                videos,
                                generation,
                                weak_for_images,
                                state_for_images,
                                handle_for_images,
                            );
                        });
                    }

                    Err(error) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let current = state
                                .lock()
                                .map(|s| s.search_generation)
                                .unwrap_or_default();

                            if current != generation {
                                return;
                            }

                            ui.set_loading(false);
                            ui.set_status_text(format!("Search failed: {error}").into());
                        });
                    }
                }
            });
        });
    }

    // ========================================
    // LOAD MORE — NEXT PAGE
    // ========================================
    {
        let weak = ui.as_weak();
        let runtime = runtime.clone();
        let state = state.clone();
        let tokio_handle = tokio_handle.clone();

        ui.on_load_more(move || {
            let request = {
                let mut s = state.lock().unwrap();

                if s.loading_more {
                    None
                } else {
                    s.continuation_token.clone().map(|token| {
                        s.loading_more = true;

                        (s.current_query.clone(), token, s.search_generation)
                    })
                }
            };

            let Some((query, token, generation)) = request else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                ui.set_loading_more(true);
            }

            let weak = weak.clone();
            let state = state.clone();
            let handle_for_images = tokio_handle.clone();

            runtime.spawn(async move {
                let result = NativeYoutubeProvider::new()
                    .search_page(&query, Some(&token))
                    .await;

                match result {
                    Ok(page) => {
                        let weak_for_images = weak.clone();
                        let state_for_images = state.clone();

                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.loading_more = false;

                            // Hindari duplikasi video jika
                            // halaman mengandung ID yang sama.
                            let new_videos = page
                                .videos
                                .into_iter()
                                .filter(|video| {
                                    !s.search_results
                                        .iter()
                                        .any(|existing| existing.id == video.id)
                                })
                                .collect::<Vec<_>>();

                            let has_new = !new_videos.is_empty();

                            // Hindari continuation yang berulang.
                            s.continuation_token = if has_new
                                && page.next_page_token.as_deref() != Some(token.as_str())
                            {
                                page.next_page_token
                            } else {
                                None
                            };

                            s.search_results.extend(new_videos.iter().cloned());

                            let total = s.search_results.len();

                            let has_more = s.continuation_token.is_some();

                            drop(s);

                            let model = ui.get_videos();

                            let mut items = (0..model.row_count())
                                .filter_map(|i| model.row_data(i))
                                .collect::<Vec<_>>();

                            items.extend(
                                new_videos
                                    .iter()
                                    .map(|video| video_to_ui(video, slint::Image::default())),
                            );

                            ui.set_videos(ModelRc::from(Rc::new(VecModel::from(items))));

                            ui.set_loading_more(false);
                            ui.set_has_more(has_more);
                            ui.set_status_text(format!("{total} videos found").into());

                            queue_thumbnails(
                                new_videos,
                                generation,
                                weak_for_images,
                                state_for_images,
                                handle_for_images,
                            );
                        });
                    }

                    Err(error) => {
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let mut s = state.lock().unwrap();

                            if s.search_generation != generation {
                                return;
                            }

                            s.loading_more = false;

                            ui.set_loading_more(false);
                            ui.set_status_text(format!("Load More failed: {error}").into());
                        });
                    }
                }
            });
        });
    }

    // ========================================
    // OPEN VIDEO
    // ========================================
    {
        let weak = ui.as_weak();
        let state = state.clone();

        ui.on_open_video(move |video_id| {
            let video = state.lock().ok().and_then(|s| {
                s.search_results
                    .iter()
                    .find(|video| video.id == video_id.as_str())
                    .cloned()
            });

            let Some(video) = video else {
                return;
            };

            if let Some(ui) = weak.upgrade() {
                ui.set_selected_video_id(video.id.into());
                ui.set_selected_video_title(video.title.into());
                ui.set_selected_video_channel(video.channel.into());
                ui.set_current_page("watch".into());
            }
        });
    }

    // ========================================
    // PLAY VIDEO
    // ========================================
    ui.on_play_video(move |video_id| {
        if let Err(error) = ExternalMpvPlayer::play_youtube(video_id.as_str()) {
            eprintln!("Player error: {error}");
        }
    });

    ui.run()
}

use std::rc::Rc;

use mutsukitube_core::{ProviderError, VideoProvider};

use mutsukitube_youtube::{NativeYoutubeProvider, ProviderMode, SearchPage, YtDlpProvider};

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, VideoItem,
    app_context::AppContext,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::video_to_ui,
};

fn parse_provider_mode(value: &str) -> ProviderMode {
    match value {
        "native" => ProviderMode::Native,
        "ytdlp" => ProviderMode::YtDlp,
        _ => ProviderMode::Auto,
    }
}

async fn fetch_first_page(query: &str, mode: ProviderMode) -> Result<SearchPage, ProviderError> {
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

fn replace_search_videos(ui: &AppWindow, items: Vec<VideoItem>) {
    let model = ui.get_videos();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<VideoItem>>() {
        model.set_vec(items);
    } else {
        ui.set_videos(ModelRc::from(Rc::new(VecModel::from(items))));
    }
}

fn append_search_videos(ui: &AppWindow, items: Vec<VideoItem>) {
    let model = ui.get_videos();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<VideoItem>>() {
        model.extend(items);
    } else {
        let mut previous = (0..model.row_count())
            .filter_map(|index| model.row_data(index))
            .collect::<Vec<_>>();

        previous.extend(items);

        replace_search_videos(ui, previous);
    }
}

fn register_search(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_search(move |query| {
        let query = query.trim().to_string();

        if query.is_empty() {
            return;
        }

        let (generation, provider_mode) = {
            let Ok(mut state) = context.state.lock() else {
                return;
            };

            state.search_generation = state.search_generation.wrapping_add(1);

            state.current_query = query.clone();
            state.search_results.clear();

            state.continuation_token = None;
            state.loading_more = false;

            let provider_mode = parse_provider_mode(&state.provider_mode);

            (state.search_generation, provider_mode)
        };

        if let Some(ui) = weak.upgrade() {
            ui.set_loading(true);
            ui.set_loading_more(false);
            ui.set_has_more(false);
            ui.set_status_text("Searching...".into());

            replace_search_videos(&ui, Vec::new());
        }

        let weak = weak.clone();
        let state = context.state.clone();
        let handle = context.handle.clone();

        let thumbnail_handle = handle.clone();

        handle.spawn(async move {
            let result = fetch_first_page(&query, provider_mode).await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                if app_state.search_generation != generation {
                    return;
                }

                ui.set_loading(false);

                match result {
                    Ok(page) => {
                        let videos = page.videos;

                        let count = videos.len();

                        app_state.search_results = videos.clone();

                        app_state.continuation_token = page.next_page_token;

                        let has_more = app_state.continuation_token.is_some();

                        drop(app_state);

                        let items = videos
                            .iter()
                            .map(|video| video_to_ui(video, slint::Image::default()))
                            .collect::<Vec<_>>();

                        replace_search_videos(&ui, items);

                        ui.set_has_more(has_more);
                        ui.set_status_text(format!("{count} videos found").into());

                        queue_thumbnails(
                            videos,
                            ThumbnailTarget::Search,
                            generation,
                            ui.as_weak(),
                            state.clone(),
                            thumbnail_handle,
                        );
                    }

                    Err(error) => {
                        drop(app_state);

                        ui.set_has_more(false);
                        ui.set_status_text(format!("Search failed: {error}").into());
                    }
                }
            });
        });
    });
}

fn register_load_more(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_load_more(move || {
        let request = {
            let Ok(mut state) = context.state.lock() else {
                return;
            };

            if state.loading_more {
                None
            } else {
                let token = state.continuation_token.clone();

                token.map(|token| {
                    state.loading_more = true;

                    (state.current_query.clone(), token, state.search_generation)
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
        let state = context.state.clone();
        let handle = context.handle.clone();
        let thumbnail_handle = handle.clone();

        handle.spawn(async move {
            let result = NativeYoutubeProvider::new()
                .search_page(&query, Some(&token))
                .await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                if app_state.search_generation != generation {
                    return;
                }

                app_state.loading_more = false;
                ui.set_loading_more(false);

                match result {
                    Ok(page) => {
                        let new_videos = page
                            .videos
                            .into_iter()
                            .filter(|video| {
                                !app_state
                                    .search_results
                                    .iter()
                                    .any(|existing| existing.id == video.id)
                            })
                            .collect::<Vec<_>>();

                        let has_new = !new_videos.is_empty();

                        app_state.continuation_token =
                            if has_new && page.next_page_token.as_deref() != Some(token.as_str()) {
                                page.next_page_token
                            } else {
                                None
                            };

                        app_state.search_results.extend(new_videos.iter().cloned());

                        let total = app_state.search_results.len();

                        let has_more = app_state.continuation_token.is_some();

                        drop(app_state);

                        let items = new_videos
                            .iter()
                            .map(|video| video_to_ui(video, slint::Image::default()))
                            .collect::<Vec<_>>();

                        append_search_videos(&ui, items);

                        ui.set_has_more(has_more);

                        ui.set_status_text(format!("{total} videos found").into());

                        queue_thumbnails(
                            new_videos,
                            ThumbnailTarget::Search,
                            generation,
                            ui.as_weak(),
                            state.clone(),
                            thumbnail_handle,
                        );
                    }

                    Err(error) => {
                        drop(app_state);

                        ui.set_status_text(format!("Load More failed: {error}").into());
                    }
                }
            });
        });
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    register_search(ui, context);
    register_load_more(ui, context);
}

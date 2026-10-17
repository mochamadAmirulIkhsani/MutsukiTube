use std::rc::Rc;
use std::sync::Arc;

use slint::{ComponentHandle, ModelRc};

use crate::{
    AppWindow, app_context::AppContext, state::AppState,
};


#[cfg(target_os = "windows")]
pub fn register(
    ui: &AppWindow,
    context: &AppContext,
    session: std::rc::Rc<std::cell::RefCell<crate::native_window::session::NativeVideoSession>>,
) {
    register_search(ui, context);
    register_select(ui, context);
    register_select_episode(ui, context);
    register_quality(ui, context);
    register_play_windows(ui, context, session);
}

#[cfg(not(target_os = "windows"))]
pub fn register(ui: &AppWindow, context: &AppContext) {
    register_search(ui, context);
    register_select(ui, context);
    register_select_episode(ui, context);
    register_quality(ui, context);
    register_play_other(ui, context);
}

fn register_search(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_search(move |query| {
        let query = query.trim().to_string();

        if query.is_empty() {
            return;
        }

        let generation = {
            let Ok(mut state) = context.state.lock() else {
                return;
            };

            state.anime_generation = state.anime_generation.wrapping_add(1);
            state.anime_results.clear();
            state.anime_episodes.clear();
            state.selected_anime = None;
            state.selected_episode = None;
            state.anime_loading = true;

            state.anime_generation
        };

        if let Some(ui) = weak.upgrade() {
            ui.set_anime_loading(true);
            ui.set_anime_status_text("Searching anime...".into());
            ui.set_anime_results(ModelRc::from(Rc::new(slint::VecModel::<slint::SharedString>::default())));
            ui.set_anime_episodes(ModelRc::from(Rc::new(slint::VecModel::<slint::SharedString>::default())));
        }

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let result = manager.search_auto(&query).await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                if app_state.anime_generation != generation {
                    return;
                }

                app_state.anime_loading = false;
                ui.set_anime_loading(false);

                match result {
                    Ok(outcome) => {
                        let titles: Vec<slint::SharedString> =
                            outcome.results.iter().map(|a| a.title.clone().into()).collect();

                        app_state.anime_results = outcome.results;
                        app_state.anime_episodes.clear();
                        app_state.selected_anime = None;
                        app_state.selected_episode = None;

                        drop(app_state);

                        ui.set_anime_results(ModelRc::from(Rc::new(slint::VecModel::from(titles))));
                        ui.set_anime_episodes(ModelRc::from(Rc::new(slint::VecModel::<slint::SharedString>::default())));
                        ui.set_selected_anime_title("".into());
                        ui.set_selected_episode_title("".into());

                        if let Some(provider) = outcome.provider_id {
                            ui.set_anime_status_text(format!("{} results", app_state_count(&state)).into());
                            let _ = provider;
                        } else {
                            ui.set_anime_status_text("No anime found".into());
                        }
                    }
                    Err(error) => {
                        drop(app_state);
                        ui.set_anime_status_text(format!("Anime search failed: {error}").into());
                    }
                }
            });
        });
    });
}

fn app_state_count(state: &Arc<std::sync::Mutex<AppState>>) -> usize {
    state.lock().map(|s| s.anime_results.len()).unwrap_or(0)
}

fn register_select(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_select(move |index| {
        let index = index as usize;

        let anime = {
            let Ok(state) = context.state.lock() else {
                return;
            };
            state.anime_results.get(index).cloned()
        };

        let Some(anime) = anime else {
            return;
        };

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let episodes = manager.episodes(&anime.provider_id, &anime.url).await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                app_state.selected_anime = Some(anime.clone());
                app_state.selected_episode = None;
                app_state.anime_episodes.clear();

                ui.set_selected_anime_title(anime.title.clone().into());
                ui.set_selected_episode_title("".into());

                match episodes {
                    Ok(episodes) => {
                        let titles: Vec<slint::SharedString> =
                            episodes.iter().map(|e| e.title.clone().into()).collect();

                        app_state.anime_episodes = episodes;
                        app_state.anime_qualities = vec!["Auto".to_string()];

                        drop(app_state);

                        ui.set_anime_episodes(ModelRc::from(Rc::new(slint::VecModel::from(titles))));
                        ui.set_anime_qualities(ModelRc::from(Rc::new(slint::VecModel::from(vec!["Auto".into()]))));
                        ui.set_anime_status_text("Episodes loaded".into());
                    }
                    Err(error) => {
                        drop(app_state);
                        ui.set_anime_status_text(format!("Episodes failed: {error}").into());
                    }
                }
            });
        });
    });
}

fn register_select_episode(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_select_episode(move |index| {
        let index = index as usize;

        let (anime, episode) = {
            let Ok(state) = context.state.lock() else {
                return;
            };

            let episode = state.anime_episodes.get(index).cloned();
            (state.selected_anime.clone(), episode)
        };

        let Some(anime) = anime else {
            return;
        };
        let Some(episode) = episode else {
            return;
        };

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let heights = manager.heights(&anime.provider_id, &episode.url).await.unwrap_or_default();

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                app_state.selected_episode = Some(episode.clone());

                let mut qualities = vec!["Auto".to_string()];
                for height in heights {
                    qualities.push(format!("{height}p"));
                }
                app_state.anime_qualities = qualities.clone();

                drop(app_state);

                ui.set_selected_episode_title(episode.title.clone().into());

                let quality_items: Vec<slint::SharedString> =
                    qualities.into_iter().map(slint::SharedString::from).collect();

                ui.set_anime_qualities(ModelRc::from(Rc::new(slint::VecModel::from(quality_items))));
            });
        });
    });
}

fn register_quality(ui: &AppWindow, context: &AppContext) {
    let context = context.clone();

    ui.on_anime_quality_changed(move |quality| {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.anime_qualities = state.anime_qualities.clone();
        let _ = quality;
    });
}

#[cfg(target_os = "windows")]
fn register_play_windows(
    ui: &AppWindow,
    context: &AppContext,
    session: std::rc::Rc<std::cell::RefCell<crate::native_window::session::NativeVideoSession>>,
) {
    let weak = ui.as_weak();
    let context = context.clone();
    let session_for_ready = session.clone();
    let weak_for_ready = weak.clone();

    ui.on_anime_play_ready(move |url, headers| {
        let headers = decode_headers(&headers);
        session_for_ready
            .borrow_mut()
            .request_anime_playback(url.to_string(), headers);

        if let Some(ui) = weak_for_ready.upgrade() {
            ui.set_current_page("watch".into());
        }
    });

    let weak_for_play = weak.clone();

    ui.on_anime_play(move || {
        let weak = weak_for_play.clone();
        let (anime, episode, quality) = {
            let Ok(state) = context.state.lock() else {
                return;
            };

            (
                state.selected_anime.clone(),
                state.selected_episode.clone(),
                state.anime_qualities.first().cloned().unwrap_or_else(|| "Auto".to_string()),
            )
        };

        let Some(anime) = anime else {
            return;
        };
        let Some(episode) = episode else {
            return;
        };

        let selected_quality = if let Some(ui) = weak.upgrade() {
            ui.get_selected_anime_quality().to_string()
        } else {
            quality
        };

        let quality = if selected_quality == "Auto" || selected_quality.is_empty() {
            "auto".to_string()
        } else {
            selected_quality.trim_end_matches('p').to_string() + "p"
        };

        let weak = weak.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let result = manager
                .stream_with_fallback(
                    &anime,
                    &episode,
                    &quality,
                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                )
                .await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                match result {
                    Ok(outcome) => {
                        let stream = outcome.stream;

                        let mut headers = Vec::new();
                        if let Some(referer) = stream.referer {
                            headers.push(("Referer".to_string(), referer));
                        }
                        for (key, value) in stream.headers {
                            headers.push((key, value));
                        }

                        ui.set_selected_video_title(anime.title.clone().into());
                        ui.set_selected_video_channel("Anime".into());
                        ui.set_playback_position(0.0);
                        ui.set_playback_duration(0.0);
                        ui.set_player_paused(true);
                        ui.set_playback_state("loading".into());

                        ui.invoke_anime_play_ready(stream.url.into(), encode_headers(&headers).into());
                    }
                    Err(error) => {
                        ui.set_anime_status_text(format!("Playback failed: {error}").into());
                    }
                }
            });
        });
    });
}

#[cfg(not(target_os = "windows"))]
fn register_play_other(ui: &AppWindow, context: &AppContext) {
    let _ = (ui, context);
}
fn encode_headers(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_headers(value: &str) -> Vec<(String, String)> {
    value
        .split(';')
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}

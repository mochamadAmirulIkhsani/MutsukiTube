use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use mutsukitube_anime::{Anime, Episode};
use mutsukitube_plugin_updater as updater;
use mutsukitube_storage as storage;

use slint::{ComponentHandle, Model, ModelRc};

use crate::{AnimeCard, AppWindow, EpisodeItem, app_context::AppContext, state::AnimePlaying};

const SHELF_LIMIT: usize = 24;

#[cfg(target_os = "windows")]
pub fn register(
    ui: &AppWindow,
    context: &AppContext,
    session: std::rc::Rc<std::cell::RefCell<crate::native_window::session::NativeVideoSession>>,
) {
    register_home(ui, context);
    register_search(ui, context);
    register_select(ui, context);
    register_select_episode(ui, context);
    register_favorite(ui, context);
    register_play_windows(ui, context, session);
    register_install(ui, context);
}

#[cfg(not(target_os = "windows"))]
pub fn register(ui: &AppWindow, context: &AppContext) {
    register_home(ui, context);
    register_search(ui, context);
    register_select(ui, context);
    register_select_episode(ui, context);
    register_favorite(ui, context);
    register_play_other(ui, context);
    register_install(ui, context);
}

// ---------------------------------------------------------------- helpers

fn format_seconds(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

fn anime_from_parts(provider_id: &str, anime_url: &str, title: &str, thumbnail_url: &str) -> Anime {
    Anime {
        provider_id: provider_id.to_string(),
        title: title.to_string(),
        url: anime_url.to_string(),
        thumbnail_url: if thumbnail_url.trim().is_empty() {
            None
        } else {
            Some(thumbnail_url.to_string())
        },
        alt_titles: Vec::new(),
        year: None,
    }
}

fn base_card(
    key: &str,
    anime: &Anime,
    subtitle: String,
    badge: String,
    progress: f32,
) -> AnimeCard {
    AnimeCard {
        key: key.into(),
        provider_id: anime.provider_id.clone().into(),
        url: anime.url.clone().into(),
        title: anime.title.clone().into(),
        subtitle: subtitle.into(),
        badge: badge.into(),
        progress,
        has_poster: false,
        poster: slint::Image::default(),
    }
}

type CardSelector = fn(&AppWindow) -> ModelRc<AnimeCard>;

fn cards_model(cards: Vec<AnimeCard>) -> ModelRc<AnimeCard> {
    ModelRc::from(Rc::new(slint::VecModel::from(cards)))
}

/// Queue poster downloads for a set of (key, poster_url) and patch them into
/// whichever Slint model the selector returns. Runs off the UI thread.
fn load_card_posters(
    weak: slint::Weak<AppWindow>,
    handle: tokio::runtime::Handle,
    selector: CardSelector,
    posters: Vec<(String, String)>,
) {
    if posters.is_empty() {
        return;
    }

    handle.spawn(async move {
        use futures::stream::{self, StreamExt};

        stream::iter(posters)
            .for_each_concurrent(6, |(key, url)| {
                let weak = weak.clone();

                async move {
                    let Some(bytes) = crate::thumbnail::download_thumbnail(&url).await else {
                        return;
                    };

                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        let Ok(image) = slint::Image::load_from_data(&bytes, None) else {
                            return;
                        };

                        let model = selector(&ui);

                        for index in 0..model.row_count() {
                            let Some(mut card) = model.row_data(index) else {
                                continue;
                            };

                            if card.key.as_str() != key {
                                continue;
                            }

                            card.has_poster = true;
                            card.poster = image;
                            model.set_row_data(index, card);
                            break;
                        }
                    });
                }
            })
            .await;
    });
}

fn is_plugins_ready(context: &AppContext) -> bool {
    !context.anime_manager.available_provider_ids().is_empty()
}

fn build_episode_items(
    episodes: &[Episode],
    progress: &HashMap<i32, storage::AnimeProgressRecord>,
    _selected_number: i32,
    playing_number: Option<i32>,
) -> Vec<EpisodeItem> {
    episodes
        .iter()
        .map(|episode| {
            let record = progress.get(&episode.number);

            let watched = record.map(|r| r.completed).unwrap_or(false);

            let status = match record {
                Some(r) if !r.completed && r.position_secs > 0 => {
                    format!("Resume {}", format_seconds(r.position_secs))
                }
                _ => String::new(),
            };

            let duration = record
                .filter(|r| r.duration_secs > 0)
                .map(|r| format_seconds(r.duration_secs))
                .unwrap_or_default();

            let playing = playing_number == Some(episode.number);

            EpisodeItem {
                number: episode.number,
                title: episode.title.clone().into(),
                status: status.into(),
                duration: duration.into(),
                watched,
                playing,
                has_thumbnail: false,
                thumbnail: slint::Image::default(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------- home

fn register_home(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let weak_back = ui.as_weak();
    let context = context.clone();
    let context_back = context.clone();

    ui.on_show_anime(move || {
        refresh_home(weak.clone(), context.clone(), None);
    });

    ui.on_anime_back(move || {
        let view = context_back
            .state
            .lock()
            .map(|s| s.anime_view.clone())
            .unwrap_or_else(|_| "home".to_string());

        if view == "details" {
            if let Some(ui) = weak_back.upgrade() {
                ui.set_anime_view("home".into());
                ui.set_selected_anime_title("".into());
            }

            if let Ok(mut state) = context_back.state.lock() {
                state.anime_view = "home".to_string();
                state.anime_selected_episode_number = -1;
            }

            refresh_home(weak_back.clone(), context_back.clone(), None);
        } else if let Some(ui) = weak_back.upgrade() {
            ui.set_current_page("search".into());
        }
    });
}

fn refresh_home(weak: slint::Weak<AppWindow>, context: AppContext, _status: Option<String>) {
    let Some(ui) = weak.upgrade() else {
        return;
    };

    let plugins_ready = is_plugins_ready(&context);

    ui.set_anime_plugins_ready(plugins_ready);

    if !plugins_ready {
        ui.set_anime_view("setup".into());

        if let Ok(mut state) = context.state.lock() {
            state.anime_view = "setup".to_string();
        }

        return;
    }

    let continue_records = storage::get_continue_watching(SHELF_LIMIT).unwrap_or_default();

    let history_records = storage::get_anime_history(SHELF_LIMIT).unwrap_or_default();

    let favorite_records = storage::get_anime_favorites(SHELF_LIMIT).unwrap_or_default();

    let mut cards_by_key: HashMap<String, Anime> = HashMap::new();
    let mut resume_map: HashMap<String, (i32, u64)> = HashMap::new();

    let mut continue_cards = Vec::new();
    let mut continue_posters = Vec::new();

    for record in &continue_records {
        let key = storage::anime_key(&record.provider_id, &record.anime_url);

        let anime = anime_from_parts(
            &record.provider_id,
            &record.anime_url,
            &record.title,
            &record.thumbnail_url,
        );

        let progress = if record.duration_secs > 0 {
            (record.position_secs as f32 / record.duration_secs as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let subtitle = format!(
            "EP {} · {}",
            record.episode_number,
            format_seconds(record.position_secs)
        );

        continue_cards.push(base_card(
            &key,
            &anime,
            subtitle,
            format!("EP {}", record.episode_number),
            progress,
        ));

        continue_posters.push((key.clone(), anime.thumbnail_url.clone().unwrap_or_default()));
        cards_by_key.insert(key.clone(), anime);
        resume_map.insert(key, (record.episode_number, record.position_secs));
    }

    let mut recent_cards = Vec::new();
    let mut recent_posters = Vec::new();

    for record in &history_records {
        let key = storage::anime_key(&record.provider_id, &record.anime_url);

        let anime = anime_from_parts(
            &record.provider_id,
            &record.anime_url,
            &record.title,
            &record.thumbnail_url,
        );

        if !cards_by_key.contains_key(&key) {
            cards_by_key.insert(key.clone(), anime.clone());
        }

        recent_cards.push(base_card(&key, &anime, String::new(), String::new(), 0.0));
        recent_posters.push((key, anime.thumbnail_url.clone().unwrap_or_default()));
    }

    let mut favorite_cards = Vec::new();
    let mut favorite_posters = Vec::new();

    for record in &favorite_records {
        let key = storage::anime_key(&record.provider_id, &record.anime_url);

        let anime = anime_from_parts(
            &record.provider_id,
            &record.anime_url,
            &record.title,
            &record.thumbnail_url,
        );

        if !cards_by_key.contains_key(&key) {
            cards_by_key.insert(key.clone(), anime.clone());
        }

        favorite_cards.push(base_card(&key, &anime, String::new(), String::new(), 0.0));
        favorite_posters.push((key, anime.thumbnail_url.clone().unwrap_or_default()));
    }

    if let Ok(mut state) = context.state.lock() {
        state.anime_view = "home".to_string();
        state.anime_cards.extend(cards_by_key);
        // Continue Watching is authoritative: rebuild the resume map so entries
        // for now-completed episodes do not linger.
        state.anime_resume = resume_map;
    }

    ui.set_anime_view("home".into());

    ui.set_anime_continue_cards(cards_model(continue_cards));
    ui.set_anime_recent_cards(cards_model(recent_cards));
    ui.set_anime_favorite_cards(cards_model(favorite_cards));

    let handle = context.handle.clone();
    load_card_posters(
        weak.clone(),
        handle.clone(),
        AppWindow::get_anime_continue_cards,
        continue_posters,
    );
    load_card_posters(
        weak.clone(),
        handle.clone(),
        AppWindow::get_anime_recent_cards,
        recent_posters,
    );
    load_card_posters(
        weak,
        handle,
        AppWindow::get_anime_favorite_cards,
        favorite_posters,
    );
}

// ---------------------------------------------------------------- search

fn register_search(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_search(move |query| {
        let query = query.trim().to_string();

        if query.is_empty() {
            return;
        }

        if !is_plugins_ready(&context) {
            if let Some(ui) = weak.upgrade() {
                ui.set_anime_view("setup".into());
                ui.set_anime_status_kind("error".into());
                ui.set_anime_status_text("Anime plugins are not installed yet".into());
            }
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
            state.anime_view = "home".to_string();
            state.anime_selected_episode_number = -1;

            state.anime_generation
        };

        if let Some(ui) = weak.upgrade() {
            ui.set_anime_loading(true);
            ui.set_anime_status_kind("info".into());
            ui.set_anime_status_text(format!("Searching for \"{query}\"...").into());
            ui.set_anime_view("home".into());
            ui.set_anime_results_heading(format!("Results for \"{query}\"").into());
            ui.set_anime_result_cards(ModelRc::from(Rc::new(slint::VecModel::from(Vec::<
                AnimeCard,
            >::new(
            )))));
        }

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();
        let poster_handle = context.handle.clone();

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
                        let mut cards = Vec::new();
                        let mut posters = Vec::new();

                        for anime in &outcome.results {
                            let key = storage::anime_key(&anime.provider_id, &anime.url);

                            app_state.anime_cards.insert(key.clone(), anime.clone());

                            let subtitle =
                                anime.year.map(|year| year.to_string()).unwrap_or_default();

                            cards.push(base_card(&key, anime, subtitle, String::new(), 0.0));

                            posters.push((key, anime.thumbnail_url.clone().unwrap_or_default()));
                        }

                        app_state.anime_results = outcome.results.clone();
                        app_state.anime_episodes.clear();
                        app_state.selected_anime = None;
                        app_state.selected_episode = None;

                        ui.set_anime_result_cards(ModelRc::from(Rc::new(slint::VecModel::from(
                            cards.clone(),
                        ))));

                        ui.set_anime_status_kind("info".into());
                        ui.set_anime_status_text(format!("{} results", cards.len()).into());

                        drop(app_state);

                        load_card_posters(
                            ui.as_weak(),
                            poster_handle.clone(),
                            AppWindow::get_anime_result_cards,
                            posters,
                        );
                    }
                    Err(error) => {
                        drop(app_state);
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(format!("Anime search failed: {error}").into());
                    }
                }
            });
        });
    });
}

// ---------------------------------------------------------------- details

fn register_select(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_select(move |key| {
        let key = key.to_string();

        let anime = context
            .state
            .lock()
            .ok()
            .and_then(|state| state.anime_cards.get(&key).cloned());

        let Some(anime) = anime else {
            return;
        };

        if let Some(ui) = weak.upgrade() {
            ui.set_anime_selected_key(key.clone().into());
            ui.set_anime_view("details".into());
            ui.set_anime_detail_title(anime.title.clone().into());
            ui.set_anime_detail_subtitle("".into());
            ui.set_anime_detail_description("".into());
            ui.set_anime_detail_badge(
                anime
                    .year
                    .map(|year| year.to_string())
                    .unwrap_or_default()
                    .into(),
            );
            ui.set_anime_detail_has_poster(false);
            ui.set_anime_detail_poster(slint::Image::default());
            ui.set_anime_detail_loading_text("Loading episodes...".into());
            ui.set_anime_episode_items(ModelRc::from(Rc::new(slint::VecModel::from(Vec::<
                EpisodeItem,
            >::new(
            )))));
            ui.set_anime_selected_episode_number(-1);
            ui.set_anime_resume_label("".into());
            ui.set_selected_anime_title(anime.title.clone().into());
        }

        let provider_id = anime.provider_id.clone();

        if let Ok(mut state) = context.state.lock() {
            state.anime_view = "details".to_string();
            state.anime_selected_key = key.clone();
            state.selected_anime = Some(anime.clone());
            state.selected_episode = None;
            state.anime_episodes.clear();
            state.anime_qualities = vec!["Auto".to_string()];
            state.anime_selected_episode_number = -1;
        }

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();
        let poster_handle = context.handle.clone();

        let poster_url = anime.thumbnail_url.clone();

        handle.spawn(async move {
            let episodes_result = manager.episodes(&provider_id, &anime.url).await;

            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_anime_detail_loading_text("".into());

                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                let mut progress: HashMap<i32, storage::AnimeProgressRecord> = HashMap::new();

                if let Ok(records) = storage::get_anime_progress_for(&provider_id, &anime.url) {
                    for record in records {
                        progress.insert(record.episode_number, record);
                    }
                }

                let resume = progress
                    .values()
                    .filter(|record| !record.completed && record.position_secs > 0)
                    .max_by_key(|record| record.updated_at);

                let (resume_number, _resume_seconds, resume_label) = match resume {
                    Some(record) => (
                        record.episode_number,
                        record.position_secs,
                        format!("Resume {}", format_seconds(record.position_secs)),
                    ),
                    None => (-1, 0, String::new()),
                };

                // Make resume work even when details were opened straight from
                // search, without a prior home refresh.
                if let Some(record) = resume {
                    app_state.anime_resume.insert(
                        storage::anime_key(&provider_id, &anime.url),
                        (record.episode_number, record.position_secs),
                    );
                }

                let selected_number = if resume_number > 0 { resume_number } else { -1 };

                match episodes_result {
                    Ok(episodes) => {
                        let items =
                            build_episode_items(&episodes, &progress, selected_number, None);

                        app_state.anime_episodes = episodes;
                        app_state.anime_selected_episode_number = selected_number;

                        if selected_number > 0 {
                            app_state.selected_episode = app_state
                                .anime_episodes
                                .iter()
                                .find(|episode| episode.number == selected_number)
                                .cloned();
                        }

                        ui.set_anime_episode_items(ModelRc::from(Rc::new(slint::VecModel::from(
                            items,
                        ))));
                        ui.set_anime_selected_episode_number(selected_number);
                        ui.set_anime_resume_label(resume_label.clone().into());

                        if selected_number > 0 {
                            ui.set_selected_episode_title(
                                app_state
                                    .selected_episode
                                    .as_ref()
                                    .map(|episode| episode.title.clone())
                                    .unwrap_or_default()
                                    .into(),
                            );
                        }

                        ui.set_anime_detail_subtitle(
                            format!("{} episodes", app_state.anime_episodes.len()).into(),
                        );
                    }
                    Err(error) => {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(format!("Episodes failed: {error}").into());
                    }
                }

                ui.set_anime_detail_favorite(
                    storage::is_anime_favorite(&provider_id, &anime.url).unwrap_or(false),
                );

                drop(app_state);

                if let Some(url) = poster_url {
                    let weak = ui.as_weak();
                    let handle = poster_handle.clone();

                    handle.spawn(async move {
                        let Some(bytes) = crate::thumbnail::download_thumbnail(&url).await else {
                            return;
                        };

                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            if let Ok(image) = slint::Image::load_from_data(&bytes, None) {
                                ui.set_anime_detail_poster(image);
                                ui.set_anime_detail_has_poster(true);
                            }
                        });
                    });
                }
            });
        });
    });
}

fn register_select_episode(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_select_episode(move |number| {
        let number = number;

        let (anime, episode) = {
            let Ok(state) = context.state.lock() else {
                return;
            };

            let episode = state
                .anime_episodes
                .iter()
                .find(|episode| episode.number == number)
                .cloned();

            let episode = episode.or_else(|| state.anime_episodes.first().cloned());

            (state.selected_anime.clone(), episode)
        };

        let Some(anime) = anime else {
            return;
        };
        let Some(episode) = episode else {
            return;
        };

        if let Ok(mut state) = context.state.lock() {
            state.selected_episode = Some(episode.clone());
            state.anime_selected_episode_number = episode.number;
        }

        if let Some(ui) = weak.upgrade() {
            ui.set_anime_selected_episode_number(episode.number);
            ui.set_selected_episode_title(episode.title.clone().into());
        }

        let weak = weak.clone();
        let state = context.state.clone();
        let manager = context.anime_manager.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let heights = manager
                .heights(&anime.provider_id, &episode.url)
                .await
                .unwrap_or_default();

            let _ = weak.upgrade_in_event_loop(move |ui| {
                let Ok(mut app_state) = state.lock() else {
                    return;
                };

                let mut qualities = vec!["Auto".to_string()];
                for height in heights {
                    qualities.push(format!("{height}p"));
                }
                app_state.anime_qualities = qualities.clone();

                ui.set_anime_qualities(ModelRc::from(Rc::new(slint::VecModel::from(
                    qualities
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                ))));
            });
        });
    });
}

fn register_favorite(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_toggle_favorite(move || {
        let anime = context
            .state
            .lock()
            .ok()
            .and_then(|state| state.selected_anime.clone());

        let Some(anime) = anime else {
            return;
        };

        let entry = storage::AnimeEntry {
            provider_id: anime.provider_id.clone(),
            anime_url: anime.url.clone(),
            title: anime.title.clone(),
            thumbnail_url: anime.thumbnail_url.clone().unwrap_or_default(),
        };

        let now_favorite = storage::toggle_anime_favorite(&entry).unwrap_or(false);

        if now_favorite {
            if let Ok(mut state) = context.state.lock() {
                let key = storage::anime_key(&entry.provider_id, &entry.anime_url);
                state.anime_cards.insert(key, anime);
            }
        }

        if let Some(ui) = weak.upgrade() {
            ui.set_anime_detail_favorite(now_favorite);
            ui.set_anime_status_kind("info".into());
            ui.set_anime_status_text(
                if now_favorite {
                    "Added to My Anime"
                } else {
                    "Removed from My Anime"
                }
                .into(),
            );
        }
    });
}

// ---------------------------------------------------------------- playback

#[cfg(target_os = "windows")]
fn register_play_windows(
    ui: &AppWindow,
    context: &AppContext,
    session: std::rc::Rc<std::cell::RefCell<crate::native_window::session::NativeVideoSession>>,
) {
    let weak_for_play = ui.as_weak();
    let context_for_play = context.clone();

    ui.on_anime_play(move || {
        let episode = context_for_play
            .state
            .lock()
            .ok()
            .and_then(|state| state.selected_episode.clone());

        let episode_number = episode.map(|e| e.number).unwrap_or(-1);

        spawn_playback(
            weak_for_play.clone(),
            context_for_play.clone(),
            episode_number,
            None,
        );
    });

    let weak_resume = ui.as_weak();
    let context_for_resume = context.clone();

    ui.on_anime_resume(move || {
        let (number, position) = context_for_resume
            .state
            .lock()
            .ok()
            .map(|state| {
                let key = state.anime_selected_key.to_string();
                let selected = state.anime_selected_episode_number;
                let resume = state.anime_resume.get(key.as_str()).copied();

                match resume {
                    Some((resume_number, seconds))
                        if selected <= 0 || selected == resume_number =>
                    {
                        (resume_number, seconds)
                    }
                    _ => (selected, 0),
                }
            })
            .unwrap_or((-1, 0));

        spawn_playback(
            weak_resume.clone(),
            context_for_resume.clone(),
            number,
            if position > 0 { Some(position) } else { None },
        );
    });

    let session_for_ready = session.clone();
    let weak_for_ready = ui.as_weak();
    let context_for_ready = context.clone();

    ui.on_anime_play_ready(move |url, headers| {
        let headers = decode_headers(&headers);
        let resume = context_for_ready
            .state
            .lock()
            .ok()
            .and_then(|state| state.anime_pending_resume);

        let mut session_ref = session_for_ready.borrow_mut();
        session_ref.request_anime_playback(url.to_string(), headers);
        session_ref.pending_anime_resume = resume.map(|seconds| seconds as f64);

        if let Some(ui) = weak_for_ready.upgrade() {
            ui.set_current_page("watch".into());
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn register_play_other(ui: &AppWindow, context: &AppContext) {
    let _ = context;

    ui.on_anime_play(|| {});
    ui.on_anime_resume(|| {});
}

#[cfg(target_os = "windows")]
fn spawn_playback(
    weak: slint::Weak<AppWindow>,
    context: AppContext,
    episode_number: i32,
    resume_seconds: Option<u64>,
) {
    let (anime, episodes, quality) = {
        let Ok(state) = context.state.lock() else {
            return;
        };

        (
            state.selected_anime.clone(),
            state.anime_episodes.clone(),
            state
                .anime_qualities
                .first()
                .cloned()
                .unwrap_or_else(|| "Auto".to_string()),
        )
    };

    let Some(anime) = anime else {
        return;
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_anime_status_kind("info".into());
        ui.set_anime_status_text("Finding a playable source...".into());
    }

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
    let state = context.state.clone();
    let manager = context.anime_manager.clone();
    let handle = context.handle.clone();

    handle.spawn(async move {
        let episodes = if episodes.is_empty() {
            match manager.episodes(&anime.provider_id, &anime.url).await {
                Ok(episodes) => episodes,
                Err(error) => {
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(format!("Playback failed: {error}").into());
                    });
                    return;
                }
            }
        } else {
            episodes
        };

        let episode = episodes
            .iter()
            .find(|episode| episode.number == episode_number)
            .or_else(|| episodes.first())
            .cloned();

        let Some(episode) = episode else {
            let _ = weak.upgrade_in_event_loop(|ui| {
                ui.set_anime_status_kind("error".into());
                ui.set_anime_status_text("No episodes available for playback".into());
            });
            return;
        };

        let result = manager
            .stream_with_fallback(
                &anime,
                &episode,
                &quality,
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
            )
            .await;

        let _ = weak.upgrade_in_event_loop(move |ui| match result {
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
                ui.set_anime_status_text("".into());

                // Commit the durable identity in the same event-loop turn that
                // asks the player to load the media. The 200 ms progress timer
                // reads `anime_now_playing`, so this keeps it impossible to
                // attribute this episode's position to a different episode.
                if let Ok(mut app_state) = state.lock() {
                    app_state.anime_pending_playing = Some(AnimePlaying {
                        provider_id: anime.provider_id.clone(),
                        anime_url: anime.url.clone(),
                        title: anime.title.clone(),
                        thumbnail_url: anime.thumbnail_url.clone().unwrap_or_default(),
                        episode_number: episode.number,
                        episode_title: episode.title.clone(),
                    });
                    app_state.anime_pending_resume = resume_seconds;
                }

                ui.invoke_anime_play_ready(stream.url.into(), encode_headers(&headers).into());
            }
            Err(error) => {
                ui.set_anime_status_kind("error".into());
                ui.set_anime_status_text(format!("Playback failed: {error}").into());
            }
        });
    });
}

// ---------------------------------------------------------------- install

fn register_install(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_anime_install_plugins(move || {
        let weak = weak.clone();
        let context = context.clone();
        let handle = context.handle.clone();

        handle.spawn(async move {
            let registry_url = match std::env::var("MUTSUKITUBE_PLUGIN_REGISTRY_URL") {
                Ok(value) if !value.trim().is_empty() => value,
                _ => {
                    let _ = weak.upgrade_in_event_loop(|ui| {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text("Plugin registry URL is not configured".into());
                    });
                    return;
                }
            };

            let public_key = updater::trusted_public_key().unwrap_or_default();

            let manifest_url = format!("{}/manifest.json", registry_url.trim_end_matches('/'));
            let signature_url = format!("{}/manifest.sig", registry_url.trim_end_matches('/'));

            let client = match reqwest::Client::builder().build() {
                Ok(client) => client,
                Err(error) => {
                    let message = format!("HTTP client error: {error}");
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(message.into());
                    });
                    return;
                }
            };

            let _ = weak.upgrade_in_event_loop(|ui| {
                ui.set_anime_status_kind("info".into());
                ui.set_anime_status_text("Checking registry...".into());
            });

            let manifest_bytes = match updater::download_artifact(
                &client,
                &manifest_url,
                updater::MAX_MANIFEST_BYTES,
            )
            .await
            {
                Ok(bytes) => bytes,
                Err(error) => {
                    let (kind, message) = updater_error("Manifest download failed", &error);
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_anime_status_kind(kind.into());
                        ui.set_anime_status_text(message.into());
                    });
                    return;
                }
            };

            let signature_bytes =
                match updater::download_artifact(&client, &signature_url, 4096).await {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        let message = format!("Signature download failed: {error}");
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_anime_status_kind("error".into());
                            ui.set_anime_status_text(message.into());
                        });
                        return;
                    }
                };

            let signature_hex = String::from_utf8_lossy(&signature_bytes).to_string();
            let manifest =
                match updater::verify_manifest(&manifest_bytes, &signature_hex, &public_key) {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        let message = format!("Manifest verification failed: {error}");
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_anime_status_kind("error".into());
                            ui.set_anime_status_text(message.into());
                        });
                        return;
                    }
                };

            let base_dir = updater::plugin_base_dir();

            if updater::is_version_failed(&base_dir, &manifest.version) {
                let version = manifest.version.clone();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_anime_status_kind("error".into());
                    ui.set_anime_status_text(
                        format!(
                            "Release {version} previously failed; not reinstalling automatically"
                        )
                        .into(),
                    );
                });
                return;
            }

            let artifacts = updater::select_artifacts(&manifest, "windows", "amd64");

            for artifact in artifacts {
                let provider = artifact.provider.clone();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_anime_status_kind("info".into());
                    ui.set_anime_status_text(format!("Downloading {provider}...").into());
                });

                let url = match updater::artifact_download_url(&manifest, artifact) {
                    Ok(url) => url,
                    Err(error) => {
                        let message = format!("Invalid artifact URL: {error}");
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            ui.set_anime_status_kind("error".into());
                            ui.set_anime_status_text(message.into());
                        });
                        return;
                    }
                };

                let bytes =
                    match updater::download_artifact(&client, &url, updater::MAX_ARTIFACT_BYTES)
                        .await
                    {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            let (kind, message) = updater_error("Download failed", &error);
                            let _ = weak.upgrade_in_event_loop(move |ui| {
                                ui.set_anime_status_kind(kind.into());
                                ui.set_anime_status_text(message.into());
                            });
                            return;
                        }
                    };

                let _ = weak.upgrade_in_event_loop(|ui| {
                    ui.set_anime_status_kind("info".into());
                    ui.set_anime_status_text("Verifying plugin...".into());
                });

                if let Err(error) = updater::verify_artifact_bytes(artifact, &bytes) {
                    let message = format!("Artifact verification failed: {error}");
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(message.into());
                    });
                    return;
                }

                let _ = weak.upgrade_in_event_loop(|ui| {
                    ui.set_anime_status_kind("info".into());
                    ui.set_anime_status_text("Installing plugin...".into());
                });

                if let Err(error) =
                    updater::install_artifact(&base_dir, &manifest.version, artifact, &bytes)
                {
                    let message = format!("Install failed: {error}");
                    let _ = weak.upgrade_in_event_loop(move |ui| {
                        ui.set_anime_status_kind("error".into());
                        ui.set_anime_status_text(message.into());
                    });
                    return;
                }
            }

            if let Err(error) = updater::activate_version(&base_dir, &manifest.version) {
                let message = format!("Activation failed: {error}");
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_anime_status_kind("error".into());
                    ui.set_anime_status_text(message.into());
                });
                return;
            }

            // Best-effort reload of the manager registry so anime becomes usable
            // without restarting when the plugin dir already existed.
            context
                .anime_manager
                .reload_registry(Some(base_dir.join(&manifest.version)));

            let version = manifest.version.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_anime_status_kind("info".into());
                ui.set_anime_status_text(format!("Plugins {version} installed").into());
            });
        });
    });
}

fn updater_error(prefix: &str, error: &updater::UpdaterError) -> (&'static str, String) {
    match error {
        updater::UpdaterError::Network(_) => {
            ("offline", "Registry offline or unreachable".to_string())
        }
        _ => ("error", format!("{prefix}: {error}")),
    }
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

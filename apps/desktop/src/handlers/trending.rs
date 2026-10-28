use std::rc::Rc;

use mutsukitube_core::Video;
use mutsukitube_youtube::YoutubeProvider;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, VideoItem,
    app_context::AppContext,
    navigation,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::video_to_ui,
};

const TRENDING_LIMIT: usize = 36;

fn replace_trending_videos(ui: &AppWindow, items: Vec<VideoItem>) {
    let model = ui.get_trending_videos();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<crate::VideoItem>>() {
        model.set_vec(items);
    } else {
        ui.set_trending_videos(ModelRc::from(Rc::new(VecModel::from(items))));
    }
}

pub fn load_trending(
    weak: slint::Weak<AppWindow>,
    context: AppContext,
    category: String,
    country: String,
) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.trending_category = category.clone();
        state.trending_country = country.clone();
        state.trending_generation = state.trending_generation.wrapping_add(1);
        state.trending_loading = true;
        state.trending_status = "Loading trending videos...".into();
        state.trending_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_trending_loading(true);
        ui.set_trending_status_text("Loading trending videos...".into());
        ui.set_trending_selected_category(category.clone().into());
        ui.set_trending_selected_country(country.clone().into());
    }

    let handle = context.handle.clone();
    let spawn_handle = handle.clone();

    spawn_handle.spawn(async move {
        let provider = YoutubeProvider::new();
        let cat_param = if category.is_empty() {
            None
        } else {
            Some(category.as_str())
        };

        let result = provider.trending(cat_param, Some(&country)).await;

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.trending_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            ui.set_trending_loading(false);

            match result {
                Ok(videos) => {
                    let page = videos
                        .into_iter()
                        .take(TRENDING_LIMIT)
                        .collect::<Vec<Video>>();

                    let items: Vec<VideoItem> = page
                        .iter()
                        .map(|video: &Video| video_to_ui(video, slint::Image::default()))
                        .collect();

                    if let Ok(mut state) = context.state.lock() {
                        state.trending_loading = false;
                        state.trending_status = format!("{} trending videos", items.len());
                        state.trending_results = page.clone();
                    }

                    ui.set_trending_status_text(format!("{} trending videos", items.len()).into());
                    replace_trending_videos(&ui, items);

                    queue_thumbnails(
                        page,
                        ThumbnailTarget::Trending,
                        generation,
                        ui.as_weak(),
                        context.state.clone(),
                        handle,
                    );
                }
                Err(error) => {
                    eprintln!("[MutsukiTube] Trending fetch error: {error}");

                    if let Ok(mut state) = context.state.lock() {
                        state.trending_loading = false;
                        state.trending_status = format!("Failed to load trending: {error}");
                        state.trending_results.clear();
                    }

                    ui.set_trending_status_text(
                        format!("Failed to load trending: {error}").into(),
                    );
                    replace_trending_videos(&ui, Vec::new());
                }
            }
        });
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    register_nav(ui, context);
    register_category(ui, context);
    register_country(ui, context);
    register_refresh(ui, context);
}

fn register_nav(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_nav_trending(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        navigation::push_navigation(
            &ui,
            &context,
            navigation::NavEntry::new(navigation::NavRoute::YouTubeTrending {
                category: String::new(),
            }),
        );
    });
}

fn register_category(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_load_trending_category(move |category| {
        let category = category.to_string();
        let country = weak
            .upgrade()
            .map(|ui| ui.get_trending_selected_country().to_string())
            .unwrap_or_else(|| "ID".into());

        load_trending(weak.clone(), context.clone(), category, country);
    });
}

fn register_country(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_toggle_trending_country(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        let current = ui.get_trending_selected_country().to_string();
        let next = if current == "ID" { "US" } else { "ID" };
        ui.set_trending_selected_country(next.into());

        let category = ui.get_trending_selected_category().to_string();

        load_trending(weak.clone(), context.clone(), category, next.into());
    });
}

fn register_refresh(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_load_trending(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        let category = ui.get_trending_selected_category().to_string();
        let country = ui.get_trending_selected_country().to_string();

        load_trending(weak.clone(), context.clone(), category, country);
    });
}

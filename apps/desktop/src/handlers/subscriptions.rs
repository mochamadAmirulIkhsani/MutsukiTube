use std::rc::Rc;

use mutsukitube_core::{Video, VideoProvider};
use mutsukitube_storage::{
    SubscribedChannel, get_subscriptions, remove_subscription,
};
use mutsukitube_youtube::YoutubeProvider;

use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::{
    AppWindow, ChannelItem, VideoItem,
    app_context::AppContext,
    navigation,
    thumbnail_ui::{ThumbnailTarget, queue_thumbnails},
    ui_models::{channel_to_ui, video_to_ui},
};

const CHANNEL_VIDEOS_LIMIT: usize = 36;

fn replace_subscriptions_channels(ui: &AppWindow, items: Vec<ChannelItem>) {
    let model = ui.get_subscriptions_channels();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<crate::ChannelItem>>() {
        model.set_vec(items);
    } else {
        ui.set_subscriptions_channels(ModelRc::from(Rc::new(VecModel::from(items))));
    }
}

fn replace_channel_videos(ui: &AppWindow, items: Vec<VideoItem>) {
    let model = ui.get_subscriptions_channel_videos();

    if let Some(model) = model.as_any().downcast_ref::<VecModel<crate::VideoItem>>() {
        model.set_vec(items);
    } else {
        ui.set_subscriptions_channel_videos(ModelRc::from(Rc::new(VecModel::from(items))));
    }
}

pub fn load_subscriptions(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_loading = true;
        state.subscriptions_status = "Loading subscriptions...".into();
        state.subscriptions_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_subscriptions_loading(true);
        ui.set_subscriptions_status_text("Loading subscriptions...".into());
    }

    let spawn_handle = context.handle.clone();

    spawn_handle.spawn(async move {
        let channels = tokio::task::spawn_blocking(get_subscriptions)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.subscriptions_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            let filter_query = context
                .state
                .lock()
                .map(|state| state.subscriptions_filter_query.clone())
                .unwrap_or_default();

            let filtered_channels: Vec<SubscribedChannel> = if filter_query.is_empty() {
                channels.clone()
            } else {
                let q = filter_query.to_lowercase();
                channels
                    .iter()
                    .filter(|c| c.title.to_lowercase().contains(&q))
                    .cloned()
                    .collect()
            };

            let items: Vec<ChannelItem> = filtered_channels
                .iter()
                .map(channel_to_ui)
                .collect();

            if let Ok(mut state) = context.state.lock() {
                state.subscriptions_loading = false;
                state.subscriptions_status = format!("{} channels", channels.len());
                state.subscriptions_all = channels.clone();
                state.subscriptions_filtered = filtered_channels;
            }

            ui.set_subscriptions_loading(false);
            ui.set_subscriptions_count(channels.len() as i32);
            ui.set_subscriptions_status_text(format!("{} channels", channels.len()).into());
            replace_subscriptions_channels(&ui, items);
        });
    });
}

pub fn filter_subscriptions(weak: slint::Weak<AppWindow>, context: AppContext, query: String) {
    if let Ok(mut state) = context.state.lock() {
        state.subscriptions_filter_query = query.clone();
    }

    if let Some(ui) = weak.upgrade() {
        let all = context
            .state
            .lock()
            .map(|s| s.subscriptions_all.clone())
            .unwrap_or_default();

        let filtered: Vec<SubscribedChannel> = if query.trim().is_empty() {
            all
        } else {
            let q = query.to_lowercase();
            all.into_iter()
                .filter(|c| c.title.to_lowercase().contains(&q))
                .collect()
        };

        let items: Vec<ChannelItem> = filtered.iter().map(channel_to_ui).collect();
        replace_subscriptions_channels(&ui, items);
    }
}

pub fn select_channel(
    weak: slint::Weak<AppWindow>,
    context: AppContext,
    channel_id: String,
    channel_title: String,
) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };

        state.subscriptions_selected_channel_id = Some(channel_id.clone());
        state.subscriptions_selected_channel_title = Some(channel_title.clone());
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_loading = true;
        state.subscriptions_status = format!("Loading videos from {channel_title}...");
        state.subscriptions_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_subscriptions_loading(true);
        ui.set_subscriptions_status_text(
            format!("Loading videos from {channel_title}...").into(),
        );
        ui.set_subscriptions_selected_channel_id(channel_id.clone().into());
        ui.set_subscriptions_selected_channel_title(channel_title.clone().into());
        replace_channel_videos(&ui, Vec::new());
    }

    let handle = context.handle.clone();
    let spawn_handle = handle.clone();
    let query = channel_title.clone();

    spawn_handle.spawn(async move {
        let provider = YoutubeProvider::new();
        let result = provider.search(&query).await;

        let _ = weak.upgrade_in_event_loop(move |ui| {
            let is_current = context
                .state
                .lock()
                .map(|state| state.subscriptions_generation == generation)
                .unwrap_or(false);

            if !is_current {
                return;
            }

            ui.set_subscriptions_loading(false);

            match result {
                Ok(videos) => {
                    let page: Vec<Video> = videos
                        .into_iter()
                        .take(CHANNEL_VIDEOS_LIMIT)
                        .collect();

                    let items: Vec<VideoItem> = page
                        .iter()
                        .map(|v| video_to_ui(v, slint::Image::default()))
                        .collect();

                    if let Ok(mut state) = context.state.lock() {
                        state.subscriptions_loading = false;
                        state.subscriptions_status = format!("{} videos", items.len());
                        state.subscriptions_channel_videos = page.clone();
                    }

                    ui.set_subscriptions_status_text(format!("{} videos", items.len()).into());
                    replace_channel_videos(&ui, items);

                    queue_thumbnails(
                        page,
                        ThumbnailTarget::Subscriptions,
                        generation,
                        ui.as_weak(),
                        context.state.clone(),
                        handle,
                    );
                }
                Err(error) => {
                    eprintln!("[MutsukiTube] Channel videos fetch error: {error}");

                    if let Ok(mut state) = context.state.lock() {
                        state.subscriptions_loading = false;
                        state.subscriptions_status = format!("Failed to load videos: {error}");
                        state.subscriptions_channel_videos.clear();
                    }

                    ui.set_subscriptions_status_text(
                        format!("Failed to load videos: {error}").into(),
                    );
                    replace_channel_videos(&ui, Vec::new());
                }
            }
        });
    });
}

pub fn clear_selected_channel(weak: slint::Weak<AppWindow>, context: AppContext) {
    if let Ok(mut state) = context.state.lock() {
        state.subscriptions_selected_channel_id = None;
        state.subscriptions_selected_channel_title = None;
        state.subscriptions_channel_videos.clear();
    }

    if let Some(ui) = weak.upgrade() {
        ui.set_subscriptions_selected_channel_id("".into());
        ui.set_subscriptions_selected_channel_title("".into());
        replace_channel_videos(&ui, Vec::new());
    }
}

pub fn remove_channel(weak: slint::Weak<AppWindow>, context: AppContext, channel_id: String) {
    let handle = context.handle.clone();
    let weak_clone = weak.clone();
    let context_clone = context.clone();

    handle.spawn_blocking(move || {
        let _ = remove_subscription(&channel_id);
        load_subscriptions(weak_clone, context_clone);
    });
}

pub fn register(ui: &AppWindow, context: &AppContext) {
    register_nav(ui, context);
    register_load(ui, context);
    register_filter(ui, context);
    register_channel_select(ui, context);
    register_channel_remove(ui, context);
}

fn register_nav(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_nav_subscriptions(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };

        navigation::push_navigation(
            &ui,
            &context,
            navigation::NavEntry::new(navigation::NavRoute::YouTubeSubscriptions {
                channel_id: None,
                channel_title: None,
            }),
        );
    });
}

fn register_load(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_load_subscriptions({
        let weak = weak.clone();
        let context = context.clone();
        move || {
            load_subscriptions(weak.clone(), context.clone());
        }
    });

    ui.on_refresh_subscriptions(move || {
        load_subscriptions(weak.clone(), context.clone());
    });
}

fn register_filter(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_filter_subscriptions(move |query| {
        filter_subscriptions(weak.clone(), context.clone(), query.to_string());
    });
}

fn register_channel_select(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_select_subscription_channel({
        let weak = weak.clone();
        let context = context.clone();
        move |id, title| {
            let id = id.to_string();
            let title = title.to_string();

            if let Some(ui) = weak.upgrade() {
                navigation::push_navigation(
                    &ui,
                    &context,
                    navigation::NavEntry::new(navigation::NavRoute::YouTubeSubscriptions {
                        channel_id: Some(id.clone()),
                        channel_title: Some(title.clone()),
                    }),
                );
            }

            select_channel(weak.clone(), context.clone(), id, title);
        }
    });

    ui.on_clear_selected_subscription_channel(move || {
        clear_selected_channel(weak.clone(), context.clone());
    });
}

fn register_channel_remove(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_remove_subscription_channel(move |id| {
        remove_channel(weak.clone(), context.clone(), id.to_string());
    });
}

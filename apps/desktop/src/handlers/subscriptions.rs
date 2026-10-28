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
    thumbnail_ui::{AvatarTarget, ThumbnailTarget, queue_channel_avatars, queue_thumbnails},
    ui_models::{channel_to_ui, video_to_ui},
};

pub const CHANNELS_PER_PAGE: usize = 24;
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

pub fn get_filtered_and_sorted(
    channels: &[SubscribedChannel],
    query: &str,
    sort: &str,
) -> Vec<SubscribedChannel> {
    let mut list: Vec<SubscribedChannel> = if query.trim().is_empty() {
        channels.to_vec()
    } else {
        let q = query.to_lowercase();
        channels
            .iter()
            .filter(|c| c.title.to_lowercase().contains(&q))
            .cloned()
            .collect()
    };

    match sort {
        "z-a" => {
            list.sort_by(|a, b| b.title.to_lowercase().cmp(&a.title.to_lowercase()));
        }
        "with-avatar" => {
            list.retain(|c| c.avatar_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false));
            list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
        _ => {
            // "a-z" default
            list.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }
    }

    list
}

pub fn refresh_subscriptions_view(
    weak: slint::Weak<AppWindow>,
    context: &AppContext,
    generation: u64,
) {
    let (all_channels, query, sort, current_page) = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        let page = if state.subscriptions_current_page == 0 {
            state.subscriptions_current_page = 1;
            1
        } else {
            state.subscriptions_current_page
        };
        let sort = if state.subscriptions_sort.is_empty() {
            state.subscriptions_sort = "a-z".to_string();
            "a-z".to_string()
        } else {
            state.subscriptions_sort.clone()
        };
        (
            state.subscriptions_all.clone(),
            state.subscriptions_filter_query.clone(),
            sort,
            page,
        )
    };

    let filtered = get_filtered_and_sorted(&all_channels, &query, &sort);
    let total_count = filtered.len();
    let total_pages = if total_count == 0 {
        1
    } else {
        (total_count + CHANNELS_PER_PAGE - 1) / CHANNELS_PER_PAGE
    };
    let page = current_page.clamp(1, total_pages);

    if let Ok(mut state) = context.state.lock() {
        state.subscriptions_current_page = page;
        state.subscriptions_filtered = filtered.clone();
    }

    let start_index = (page - 1) * CHANNELS_PER_PAGE;
    let page_channels: Vec<SubscribedChannel> = filtered
        .into_iter()
        .skip(start_index)
        .take(CHANNELS_PER_PAGE)
        .collect();
    let end_index = start_index + page_channels.len();

    let weak_clone = weak.clone();
    let context_clone = context.clone();

    let _ = weak.upgrade_in_event_loop(move |ui| {
        let is_current = context_clone
            .state
            .lock()
            .map(|state| state.subscriptions_generation == generation)
            .unwrap_or(false);

        if !is_current {
            return;
        }

        let items: Vec<ChannelItem> = page_channels
            .iter()
            .map(|c| channel_to_ui(c, slint::Image::default()))
            .collect();

        ui.set_subscriptions_loading(false);
        ui.set_subscriptions_count(all_channels.len() as i32);
        ui.set_subscriptions_current_page(page as i32);
        ui.set_subscriptions_total_pages(total_pages as i32);
        ui.set_subscriptions_total_count(total_count as i32);
        ui.set_subscriptions_page_start(if total_count == 0 { 0 } else { (start_index + 1) as i32 });
        ui.set_subscriptions_page_end(end_index as i32);
        ui.set_subscriptions_sort(sort.into());
        ui.set_subscriptions_status_text(format!("{} total channels", all_channels.len()).into());
        replace_subscriptions_channels(&ui, items);

        queue_channel_avatars(
            page_channels,
            AvatarTarget::Subscriptions,
            generation,
            weak_clone,
            context_clone.state.clone(),
            context_clone.handle.clone(),
        );
    });
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
    let context_clone = context.clone();
    let weak_clone = weak.clone();

    spawn_handle.spawn(async move {
        let channels = tokio::task::spawn_blocking(get_subscriptions)
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or_default();

        if let Ok(mut state) = context_clone.state.lock() {
            state.subscriptions_all = channels;
        }

        refresh_subscriptions_view(weak_clone, &context_clone, generation);
    });
}

pub fn filter_subscriptions(weak: slint::Weak<AppWindow>, context: AppContext, query: String) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_filter_query = query;
        state.subscriptions_current_page = 1;
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
}

pub fn next_page(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_current_page += 1;
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
}

pub fn prev_page(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        if state.subscriptions_current_page > 1 {
            state.subscriptions_current_page -= 1;
        }
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
}

pub fn first_page(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_current_page = 1;
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
}

pub fn last_page(weak: slint::Weak<AppWindow>, context: AppContext) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_current_page = usize::MAX;
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
}

pub fn change_sort(weak: slint::Weak<AppWindow>, context: AppContext, sort: String) {
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_sort = sort;
        state.subscriptions_current_page = 1;
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    refresh_subscriptions_view(weak, &context, generation);
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
        ui.set_subscriptions_selected_channel_id(channel_id.clone().into());
        ui.set_subscriptions_selected_channel_title(channel_title.clone().into());
        ui.set_subscriptions_loading(true);
        ui.set_subscriptions_status_text(format!("Loading videos from {channel_title}...").into());
    }

    let spawn_handle = context.handle.clone();
    let handle = context.handle.clone();
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
    let generation = {
        let Ok(mut state) = context.state.lock() else {
            return;
        };
        state.subscriptions_selected_channel_id = None;
        state.subscriptions_selected_channel_title = None;
        state.subscriptions_channel_videos.clear();
        state.subscriptions_generation = state.subscriptions_generation.wrapping_add(1);
        state.subscriptions_generation
    };

    if let Some(ui) = weak.upgrade() {
        ui.set_subscriptions_selected_channel_id("".into());
        ui.set_subscriptions_selected_channel_title("".into());
        replace_channel_videos(&ui, Vec::new());
    }

    refresh_subscriptions_view(weak, &context, generation);
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
    register_pagination(ui, context);
    register_sort(ui, context);
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

fn register_pagination(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_subscriptions_next_page({
        let weak = weak.clone();
        let context = context.clone();
        move || {
            next_page(weak.clone(), context.clone());
        }
    });

    ui.on_subscriptions_prev_page({
        let weak = weak.clone();
        let context = context.clone();
        move || {
            prev_page(weak.clone(), context.clone());
        }
    });

    ui.on_subscriptions_first_page({
        let weak = weak.clone();
        let context = context.clone();
        move || {
            first_page(weak.clone(), context.clone());
        }
    });

    ui.on_subscriptions_last_page({
        let weak = weak.clone();
        let context = context.clone();
        move || {
            last_page(weak.clone(), context.clone());
        }
    });
}

fn register_sort(ui: &AppWindow, context: &AppContext) {
    let weak = ui.as_weak();
    let context = context.clone();

    ui.on_subscriptions_change_sort(move |sort| {
        change_sort(weak.clone(), context.clone(), sort.to_string());
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

use mutsukitube_core::Video;

use crate::{ChannelItem, VideoItem};

pub fn format_duration(seconds: Option<u64>) -> String {
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

pub fn video_to_ui(video: &Video, thumbnail: slint::Image) -> VideoItem {
    VideoItem {
        id: video.id.clone().into(),
        title: video.title.clone().into(),
        channel: video.channel.clone().into(),
        duration: format_duration(video.duration).into(),
        thumbnail,
    }
}

pub fn compute_initials(title: &str) -> String {
    let words: Vec<&str> = title.split_whitespace().collect();
    match words.as_slice() {
        [] => "?".to_string(),
        [single] => single.chars().take(2).collect::<String>().to_uppercase(),
        [first, second, ..] => {
            let c1 = first.chars().next().unwrap_or('?');
            let c2 = second.chars().next().unwrap_or('?');
            format!("{}{}", c1, c2).to_uppercase()
        }
    }
}

pub fn channel_to_ui(
    channel: &mutsukitube_storage::SubscribedChannel,
    avatar: slint::Image,
) -> ChannelItem {
    let size = avatar.size();
    let has_avatar = size.width > 0 && size.height > 0;
    ChannelItem {
        id: channel.id.clone().into(),
        url: channel.url.clone().into(),
        title: channel.title.clone().into(),
        initials: compute_initials(&channel.title).into(),
        avatar,
        has_avatar,
    }
}

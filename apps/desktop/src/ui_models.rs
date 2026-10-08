use mutsukitube_core::Video;

use crate::VideoItem;

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

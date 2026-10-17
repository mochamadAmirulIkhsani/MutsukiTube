use crate::player_controller::EmbeddedPlayerController;

use super::windows::Win32VideoSurface;

pub struct NativeVideoSession {
    pub surface: Option<Win32VideoSurface>,
    pub controller: EmbeddedPlayerController,

    pub pending_video_id: Option<String>,

    pub pending_anime_source: Option<String>,

    pub pending_anime_headers: Vec<(String, String)>,
}

impl NativeVideoSession {
    pub fn new() -> Self {
        Self {
            surface: None,
            controller: EmbeddedPlayerController::new(),
            pending_video_id: None,
            pending_anime_source: None,
            pending_anime_headers: Vec::new(),
        }
    }

    pub fn request_video(&mut self, video_id: String) {
        self.pending_video_id = Some(video_id);
    }

    pub fn request_anime_playback(&mut self, source: String, headers: Vec<(String, String)>) {
        self.pending_anime_source = Some(source);
        self.pending_anime_headers = headers;
    }

    pub fn shutdown(&mut self) {
        self.pending_video_id = None;

        self.controller.shutdown();
        self.surface.take();
    }
}

impl Drop for NativeVideoSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

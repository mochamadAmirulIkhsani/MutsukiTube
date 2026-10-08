use crate::player_controller::EmbeddedPlayerController;

use super::windows::Win32VideoSurface;

pub struct NativeVideoSession {
    pub surface: Option<Win32VideoSurface>,
    pub controller: EmbeddedPlayerController,

    // Video yang menunggu player siap.
    pub pending_video_id: Option<String>,
}

impl NativeVideoSession {
    pub fn new() -> Self {
        Self {
            surface: None,
            controller: EmbeddedPlayerController::new(),
            pending_video_id: None,
        }
    }

    pub fn request_video(&mut self, video_id: String) {
        // Permintaan terbaru menggantikan yang lama.
        self.pending_video_id = Some(video_id);
    }

    pub fn shutdown(&mut self) {
        self.pending_video_id = None;

        // libmpv dilepaskan sebelum HWND.
        self.controller.shutdown();
        self.surface.take();
    }
}

impl Drop for NativeVideoSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

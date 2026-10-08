use crate::player_controller::EmbeddedPlayerController;

use super::windows::Win32VideoSurface;

pub struct NativeVideoSession {
    pub surface: Option<Win32VideoSurface>,
    pub controller: EmbeddedPlayerController,
}

impl NativeVideoSession {
    pub fn new() -> Self {
        Self {
            surface: None,
            controller: EmbeddedPlayerController::new(),
        }
    }

    pub fn shutdown(&mut self) {
        // Lepaskan libmpv terlebih dahulu.
        self.controller.shutdown();

        // Baru hancurkan child HWND.
        self.surface.take();
    }
}

impl Drop for NativeVideoSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

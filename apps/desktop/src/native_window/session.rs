use mutsukitube_embedded_player::EmbeddedMpvPlayer;

use super::windows::Win32VideoSurface;

pub struct NativeVideoSession {
    pub surface: Option<Win32VideoSurface>,
    pub player: Option<EmbeddedMpvPlayer>,
    pub media_loaded: bool,
}

impl NativeVideoSession {
    pub fn new() -> Self {
        Self {
            surface: None,
            player: None,
            media_loaded: false,
        }
    }

    pub fn shutdown(&mut self) {
        // Hancurkan libmpv terlebih dahulu.
        self.player.take();

        // Baru kemudian hancurkan child HWND.
        self.surface.take();

        self.media_loaded = false;
    }
}

impl Drop for NativeVideoSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

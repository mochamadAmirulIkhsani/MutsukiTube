use mutsukitube_embedded_player::EmbeddedMpvPlayer;

pub struct EmbeddedPlayerController {
    player: Option<EmbeddedMpvPlayer>,
    current_source: Option<String>,
    was_on_watch_page: bool,
}

impl EmbeddedPlayerController {
    pub fn new() -> Self {
        Self {
            player: None,
            current_source: None,
            was_on_watch_page: false,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.player.is_some()
    }

    pub fn initialize(&mut self, hwnd: usize) -> Result<(), String> {
        if self.player.is_some() {
            return Ok(());
        }

        let player = EmbeddedMpvPlayer::new_for_hwnd(hwnd)?;

        self.player = Some(player);

        println!("[MutsukiTube] Player controller initialized");

        Ok(())
    }

    pub fn load_video(&mut self, source: &str) -> Result<(), String> {
        if source.trim().is_empty() {
            return Err("Video source cannot be empty".into());
        }

        let player = self
            .player
            .as_ref()
            .ok_or("Embedded player is not initialized")?;

        if self.current_source.as_deref() == Some(source) {
            return Ok(());
        }

        player.load(source)?;

        self.current_source = Some(source.to_string());

        println!("[MutsukiTube] Loading source: {source}");

        Ok(())
    }

    pub fn play(&self) -> Result<(), String> {
        self.player.as_ref().ok_or("Player not initialized")?.play()
    }

    pub fn pause(&self) -> Result<(), String> {
        self.player
            .as_ref()
            .ok_or("Player not initialized")?
            .pause()
    }

    pub fn toggle_pause(&self) -> Result<(), String> {
        let player = self.player.as_ref().ok_or("Player not initialized")?;

        if player.is_paused()? {
            player.play()
        } else {
            player.pause()
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(player) = self.player.as_ref() {
            player.stop()?;
        }

        self.current_source = None;

        Ok(())
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Invalid seek position".into());
        }

        self.player
            .as_ref()
            .ok_or("Player not initialized")?
            .seek(seconds)
    }

    pub fn set_volume(&self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Invalid volume".into());
        }

        self.player
            .as_ref()
            .ok_or("Player not initialized")?
            .set_volume(value.clamp(0.0, 100.0))
    }

    pub fn position(&self) -> Option<f64> {
        self.player.as_ref()?.position().ok()
    }

    pub fn duration(&self) -> Option<f64> {
        self.player.as_ref()?.duration().ok()
    }

    pub fn on_navigation(&mut self, is_watch_page: bool) {
        // Hanya jalankan ketika halaman berubah.
        if self.was_on_watch_page == is_watch_page {
            return;
        }

        self.was_on_watch_page = is_watch_page;

        if !is_watch_page {
            // Kebijakan sementara:
            // hentikan media saat meninggalkan WatchPage.
            if let Err(error) = self.stop() {
                eprintln!("[MutsukiTube] Navigation stop error: {error}");
            }
        }
    }

    pub fn shutdown(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("[MutsukiTube] Stop during shutdown: {error}");
        }

        // Drop libmpv sebelum HWND dihancurkan.
        self.player.take();

        self.current_source = None;
        self.was_on_watch_page = false;
    }
}

impl Default for EmbeddedPlayerController {
    fn default() -> Self {
        Self::new()
    }
}

use mutsukitube_embedded_player::{EmbeddedMpvPlayer, EmbeddedPlaybackEvent};

mod timeline;

pub use timeline::TimelineSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Loading,
    Playing,
    Paused,
    Finished,
    Stopped,
    Error,
}

impl PlaybackState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loading => "loading",
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Finished => "finished",
            Self::Stopped => "stopped",
            Self::Error => "error",
        }
    }
}

pub struct EmbeddedPlayerController {
    player: Option<EmbeddedMpvPlayer>,
    current_source: Option<String>,
    was_on_watch_page: bool,
    playback_state: PlaybackState,
    recovery_attempts: u32,
    max_recovery_attempts: u32,
    last_failed_source: Option<String>,
    reloading_for_recovery: bool,
    /// Whether the *current* media has finished loading. Reset on every
    /// `load_media` so the previous file's libmpv properties cannot leak into
    /// the new media's timeline.
    media_loaded: bool,
    /// Monotonic media-load counter. Increments on every `load_media`; used to
    /// reason about which load asynchronous events belong to.
    generation: u64,
}

impl EmbeddedPlayerController {
    pub fn new() -> Self {
        Self {
            player: None,
            current_source: None,
            was_on_watch_page: false,
            playback_state: PlaybackState::Idle,
            recovery_attempts: 0,
            max_recovery_attempts: 2,
            last_failed_source: None,
            reloading_for_recovery: false,
            media_loaded: false,
            generation: 0,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.player.is_some()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
    }

    pub fn initialize(&mut self, hwnd: usize) -> Result<(), String> {
        if self.player.is_some() {
            return Ok(());
        }

        match EmbeddedMpvPlayer::new_for_hwnd(hwnd) {
            Ok(player) => {
                self.player = Some(player);
                self.playback_state = PlaybackState::Idle;

                println!("[MutsukiTube] Player controller initialized");

                Ok(())
            }

            Err(error) => {
                self.playback_state = PlaybackState::Error;
                Err(error)
            }
        }
    }

    pub fn load_video(&mut self, source: &str) -> Result<(), String> {
        self.load_media(source, &[])
    }

    pub fn load_media(&mut self, source: &str, headers: &[(String, String)]) -> Result<(), String> {
        if source.trim().is_empty() {
            self.playback_state = PlaybackState::Error;

            return Err("Video source cannot be empty".into());
        }

        let Some(player) = self.player.as_ref() else {
            self.playback_state = PlaybackState::Error;

            return Err("Embedded player is not initialized".into());
        };

        if !self.reloading_for_recovery {
            self.recovery_attempts = 0;
            self.last_failed_source = None;
        }

        self.playback_state = PlaybackState::Loading;
        self.current_source = None;

        // New media session: drop any events still queued for the outgoing file
        // (e.g. a late `FileLoaded`) and mark the timeline empty until the
        // incoming file reports its own metadata.
        self.generation = self.generation.wrapping_add(1);
        self.media_loaded = false;
        player.drain_events();

        if let Err(error) = player.load_with_headers(source, headers) {
            self.playback_state = PlaybackState::Error;
            return Err(error);
        }

        self.current_source = Some(source.to_string());

        println!("[MutsukiTube] Loading source: {source}");

        Ok(())
    }

    pub fn play(&self) -> Result<(), String> {
        self.player.as_ref().ok_or("Player not initialized")?.play()
    }

    #[allow(dead_code)]
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
            if let Err(error) = player.stop() {
                self.playback_state = PlaybackState::Error;
                return Err(error);
            }
        }

        self.current_source = None;
        self.playback_state = PlaybackState::Stopped;
        self.media_loaded = false;

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

    /// Whether the media currently being loaded has finished loading.
    pub fn media_loaded(&self) -> bool {
        self.media_loaded
    }

    /// Monotonic id of the current media load. Changes on every `load_media`.
    pub fn media_generation(&self) -> u64 {
        self.generation
    }

    /// Resolved timeline for the *current* media. While loading, this is empty
    /// so the previous media's duration/position cannot reach the UI.
    pub fn timeline(&self) -> TimelineSnapshot {
        if !self.media_loaded {
            return TimelineSnapshot::default();
        }

        timeline::resolve_timeline(true, self.position(), self.duration())
    }

    pub fn is_paused(&self) -> Option<bool> {
        self.player.as_ref()?.is_paused().ok()
    }

    pub fn on_navigation(&mut self, is_watch_page: bool) {
        if self.was_on_watch_page == is_watch_page {
            return;
        }

        self.was_on_watch_page = is_watch_page;

        if !is_watch_page {
            if let Err(error) = self.stop() {
                eprintln!("[MutsukiTube] Navigation stop error: {error}");
            }
        }
    }

    pub fn poll_events(&mut self) {
        let Some(player) = self.player.as_ref() else {
            return;
        };

        let events = player.poll_events();

        let mut recovery_needed = false;

        for event in events {
            match event {
                EmbeddedPlaybackEvent::FileLoaded => {
                    if self.current_source.is_none() {
                        continue;
                    }

                    self.recovery_attempts = 0;
                    self.last_failed_source = None;

                    // The incoming file's own metadata is now authoritative.
                    self.media_loaded = true;

                    let paused = player.is_paused().unwrap_or(false);

                    self.playback_state = if paused {
                        PlaybackState::Paused
                    } else {
                        PlaybackState::Playing
                    };

                    println!(
                        "[MutsukiTube] File loaded: {}",
                        self.playback_state.as_str()
                    );
                }

                EmbeddedPlaybackEvent::EndOfFile => {
                    if self.current_source.is_some()
                        && self.playback_state != PlaybackState::Loading
                    {
                        self.playback_state = PlaybackState::Finished;

                        println!("[MutsukiTube] Playback finished");
                    }
                }

                EmbeddedPlaybackEvent::Stopped => {
                    if !matches!(
                        self.playback_state,
                        PlaybackState::Loading | PlaybackState::Finished | PlaybackState::Error
                    ) {
                        self.playback_state = PlaybackState::Stopped;
                    }
                }

                EmbeddedPlaybackEvent::Error => {
                    self.playback_state = PlaybackState::Error;

                    eprintln!("[MutsukiTube] Playback error event");

                    if self.recovery_attempts < self.max_recovery_attempts
                        && self.last_failed_source != self.current_source
                    {
                        recovery_needed = true;
                    }
                }
            }
        }

        if recovery_needed {
            self.recovery_attempts += 1;
            self.last_failed_source = self.current_source.clone();

            if let Err(error) = self.replay() {
                eprintln!("[MutsukiTube] Recovery replay failed: {error}");
            }
        }

        if matches!(
            self.playback_state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            if let Some(player) = self.player.as_ref() {
                if let Ok(paused) = player.is_paused() {
                    self.playback_state = if paused {
                        PlaybackState::Paused
                    } else {
                        PlaybackState::Playing
                    };
                }
            }
        }
    }

    pub fn replay(&mut self) -> Result<(), String> {
        let source = self.current_source.clone().ok_or("No media loaded")?;

        self.reloading_for_recovery = true;
        let result = self.load_video(&source);
        self.reloading_for_recovery = false;

        result?;

        if let Err(error) = self.play() {
            self.playback_state = PlaybackState::Error;
            return Err(error);
        }

        println!("[MutsukiTube] Replaying current video");

        Ok(())
    }

    pub fn shutdown(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("[MutsukiTube] Stop during shutdown: {error}");
        }

        self.player.take();

        self.current_source = None;
        self.was_on_watch_page = false;
        self.playback_state = PlaybackState::Idle;
        self.recovery_attempts = 0;
        self.last_failed_source = None;
        self.reloading_for_recovery = false;
        self.media_loaded = false;
        self.generation = 0;
    }
}

impl Default for EmbeddedPlayerController {
    fn default() -> Self {
        Self::new()
    }
}

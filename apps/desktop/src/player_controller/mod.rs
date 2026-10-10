use mutsukitube_embedded_player::{EmbeddedMpvPlayer, EmbeddedPlaybackEvent};

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
}

impl EmbeddedPlayerController {
    pub fn new() -> Self {
        Self {
            player: None,
            current_source: None,
            was_on_watch_page: false,
            playback_state: PlaybackState::Idle,
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
        if source.trim().is_empty() {
            self.playback_state = PlaybackState::Error;

            return Err("Video source cannot be empty".into());
        }

        let Some(player) = self.player.as_ref() else {
            self.playback_state = PlaybackState::Error;

            return Err("Embedded player is not initialized".into());
        };

        self.playback_state = PlaybackState::Loading;
        self.current_source = None;

        if let Err(error) = player.load(source) {
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

        for event in events {
            match event {
                EmbeddedPlaybackEvent::FileLoaded => {
                    if self.current_source.is_none() {
                        continue;
                    }

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
                    if self.current_source.is_some() {
                        self.playback_state = PlaybackState::Finished;

                        println!("[MutsukiTube] Playback finished");
                    }
                }

                EmbeddedPlaybackEvent::Stopped => {
                    if self.playback_state != PlaybackState::Loading
                        && self.playback_state != PlaybackState::Finished
                    {
                        self.playback_state = PlaybackState::Stopped;
                    }
                }

                EmbeddedPlaybackEvent::Error => {
                    if self.playback_state != PlaybackState::Loading {
                        self.playback_state = PlaybackState::Error;
                    }

                    eprintln!("[MutsukiTube] Playback error event");
                }
            }
        }

        if matches!(
            self.playback_state,
            PlaybackState::Playing | PlaybackState::Paused
        ) {
            if let Ok(paused) = player.is_paused() {
                self.playback_state = if paused {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Playing
                };
            }
        }
    }

    pub fn replay(&mut self) -> Result<(), String> {
        let source = self.current_source.clone().ok_or("No media loaded")?;

        self.load_video(&source)?;

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
    }
}

impl Default for EmbeddedPlayerController {
    fn default() -> Self {
        Self::new()
    }
}

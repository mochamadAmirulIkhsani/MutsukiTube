use rsmpv::{EndFileReason, Event, Mpv};

pub struct EmbeddedMpvPlayer {
    mpv: Mpv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedPlaybackEvent {
    FileLoaded,
    EndOfFile,
    Stopped,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_without_headers_does_not_return_invalid_parameter() {
        let player = EmbeddedMpvPlayer::new_headless().unwrap();
        player.load("https://example.com/video.mp4").unwrap();
    }

    #[test]
    fn load_with_headers_does_not_return_invalid_parameter() {
        let player = EmbeddedMpvPlayer::new_headless().unwrap();
        player
            .load_with_headers(
                "https://example.com/video.mp4",
                &[("Referer".to_string(), "https://example.com/".to_string())],
            )
            .unwrap();
    }

    #[test]
    fn test_ytdl_raw_options() {
        let player = EmbeddedMpvPlayer::new_headless().unwrap();
        let cmd_res = player.mpv.command(&[
            "change-list",
            "script-opts",
            "append",
            "ytdl_hook-ytdl_args=--extractor-args=youtube:player_client=android,ios,web",
        ]);
        println!("command change-list script-opts result: {:?}", cmd_res);
        assert!(cmd_res.is_ok());
    }
}

impl EmbeddedMpvPlayer {
    pub fn new_headless() -> Result<Self, String> {
        let mpv = Mpv::builder()
            .map_err(|error| error.to_string())?
            .set_property("vo", "null")
            .map_err(|error| error.to_string())?
            .set_property("ao", "null")
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string())?;

        let _ = mpv.command(&[
            "change-list",
            "script-opts",
            "append",
            "ytdl_hook-ytdl_args=--extractor-args=youtube:player_client=android,ios,web",
        ]);

        Ok(Self { mpv })
    }

    pub fn load(&self, url: &str) -> Result<(), String> {
        self.load_with_headers(url, &[])
    }

    pub fn load_with_headers(&self, url: &str, headers: &[(String, String)]) -> Result<(), String> {
        if headers.is_empty() {
            return self
                .mpv
                .command(&["loadfile", url, "replace"])
                .map_err(|error| error.to_string());
        }

        let header_fields = headers
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join(",");

        self.mpv
            .command(&[
                "loadfile",
                url,
                "replace",
                "-1",
                &format!("http-header-fields={header_fields}"),
            ])
            .map_err(|error| error.to_string())
    }

    pub fn play(&self) -> Result<(), String> {
        self.mpv
            .set_property("pause", false)
            .map_err(|error| error.to_string())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.mpv
            .set_property("pause", true)
            .map_err(|error| error.to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        self.mpv
            .command(&["stop"])
            .map_err(|error| error.to_string())
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Invalid seek position".into());
        }

        println!("[MutsukiTube] libmpv seek requested: {:.2}s", seconds);

        self.mpv
            .command(&["seek", &seconds.to_string(), "absolute"])
            .map_err(|error| error.to_string())?;

        println!("[MutsukiTube] libmpv seek command accepted");

        Ok(())
    }

    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.mpv
            .set_property("volume", volume.clamp(0.0, 100.0))
            .map_err(|error| error.to_string())
    }

    pub fn new_for_hwnd(hwnd: usize) -> Result<Self, String> {
        if hwnd == 0 {
            return Err("Invalid HWND".into());
        }

        let window_id = (hwnd as u32).to_string();

        let mpv = Mpv::builder()
            .map_err(|error| error.to_string())?
            .set_property("wid", window_id)
            .map_err(|error| error.to_string())?
            .set_property("hwdec", "auto-safe")
            .map_err(|error| error.to_string())?
            .set_property("force-window", "yes")
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string())?;

        let _ = mpv.command(&[
            "change-list",
            "script-opts",
            "append",
            "ytdl_hook-ytdl_args=--extractor-args=youtube:player_client=android,ios,web",
        ]);

        Ok(Self { mpv })
    }

    pub fn is_paused(&self) -> Result<bool, String> {
        self.mpv
            .get_property::<bool>("pause")
            .map_err(|error| error.to_string())
    }

    pub fn position(&self) -> Result<f64, String> {
        self.mpv
            .get_property::<f64>("playback-time")
            .map_err(|error| error.to_string())
    }

    pub fn duration(&self) -> Result<f64, String> {
        self.mpv
            .get_property::<f64>("duration")
            .map_err(|error| error.to_string())
    }

    pub fn volume(&self) -> Result<f64, String> {
        self.mpv
            .get_property::<f64>("volume")
            .map_err(|error| error.to_string())
    }

    /// Discard every queued libmpv event without translating it.
    ///
    /// Called right before `loadfile replace` so a `FileLoaded` that libmpv
    /// queued for the outgoing file cannot be mistaken for the incoming file's
    /// load (which would expose the previous media's duration prematurely).
    pub fn drain_events(&self) {
        while self.mpv.poll_event().is_some() {}
    }

    pub fn poll_events(&self) -> Vec<EmbeddedPlaybackEvent> {
        let mut events = Vec::new();

        loop {
            let Some(event) = self.mpv.poll_event() else {
                break;
            };

            match event {
                Event::FileLoaded => {
                    events.push(EmbeddedPlaybackEvent::FileLoaded);
                }

                Event::EndFile { reason, .. } => match reason {
                    EndFileReason::Eof => {
                        events.push(EmbeddedPlaybackEvent::EndOfFile);
                    }

                    EndFileReason::Stop => {
                        events.push(EmbeddedPlaybackEvent::Stopped);
                    }

                    EndFileReason::Error => {
                        events.push(EmbeddedPlaybackEvent::Error);

                        eprintln!("[MutsukiTube] libmpv failed while loading or playing media");
                    }

                    _ => {}
                },

                _ => {}
            }
        }

        events
    }
}

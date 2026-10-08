use rsmpv::{Event, Mpv};

pub struct EmbeddedMpvPlayer {
    mpv: Mpv,
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

        Ok(Self { mpv })
    }

    pub fn load(&self, url: &str) -> Result<(), String> {
        self.mpv
            .command(&["loadfile", url, "replace"])
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
        self.mpv
            .command(&["seek", &seconds.to_string(), "absolute"])
            .map_err(|error| error.to_string())
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

        // Nilai HWND harus diperlakukan sebagai ID window,
        // bukan pointer yang dapat di-dereference.
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

    pub fn poll_events(&self) {
        while let Some(event) = self.mpv.poll_event() {
            match event {
                Event::StartFile { .. } => {
                    println!("[libmpv] Starting video...");
                }

                Event::FileLoaded => {
                    println!("[libmpv] Video successfully loaded");
                }

                Event::EndFile { reason, error, .. } => {
                    eprintln!("[libmpv] Playback ended: {reason:?}, {error:?}");
                }

                Event::Shutdown => {
                    println!("[libmpv] Shutting down");
                }

                _ => {}
            }
        }
    }
}

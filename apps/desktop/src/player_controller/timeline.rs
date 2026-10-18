//! Pure playback-timeline resolution for the native Watch page.
//!
//! The Slint watch page must always describe the *currently loaded* media. After
//! `loadfile replace`, libmpv keeps reporting the outgoing file's `duration` and
//! `playback-time` for a short while (until the new file's metadata is parsed and
//! a `FileLoaded` event fires). Polling those raw values directly painted the
//! previous episode's maximum onto the incoming episode's seek slider — the
//! "slider shows 800" bug.
//!
//! Everything here is a pure function of `(media_loaded, raw_position,
//! raw_duration)`, so the decision is deterministic and unit-testable without a
//! live libmpv instance.

/// What the Watch page should display for the current media item.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TimelineSnapshot {
    /// Position to show. Always finite and `>= 0`. `0` until the current media
    /// has finished loading.
    pub position: f64,
    /// Duration to show. `0` when unknown; never a value left over from another
    /// media item.
    pub duration: f64,
    /// Whether `duration` is a real positive value for the current media, i.e.
    /// the seek slider may be enabled.
    pub has_duration: bool,
    /// Whether the current media finished loading (`FileLoaded` seen since the
    /// last `load_media`).
    pub media_loaded: bool,
}

/// Resolve the UI timeline from a media-loaded gate and raw libmpv readings.
///
/// While `media_loaded` is false the snapshot is empty: no stale position or
/// duration can reach Slint. Once loaded, only finite, in-range values are
/// accepted, the position is clamped to a known duration, and an absent or
/// non-positive duration is reported as "unknown" rather than fabricated.
pub fn resolve_timeline(
    media_loaded: bool,
    raw_position: Option<f64>,
    raw_duration: Option<f64>,
) -> TimelineSnapshot {
    if !media_loaded {
        return TimelineSnapshot::default();
    }

    let position = match raw_position {
        Some(value) if value.is_finite() && value >= 0.0 => value,
        _ => 0.0,
    };

    let duration = match raw_duration {
        Some(value) if value.is_finite() && value > 0.0 => value,
        _ => 0.0,
    };

    let has_duration = duration > 0.0;

    TimelineSnapshot {
        position: if has_duration {
            position.min(duration)
        } else {
            position
        },
        duration,
        has_duration,
        media_loaded: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenario A: episode 1 is at 620/800; episode 2 is loading. The incoming
    /// episode must never inherit the outgoing duration or position.
    #[test]
    fn switching_episodes_does_not_leak_previous_timeline() {
        // libmpv still reports episode 1's values while episode 2 loads.
        let snapshot = resolve_timeline(false, Some(620.0), Some(800.0));

        assert_eq!(snapshot.position, 0.0);
        assert_eq!(snapshot.duration, 0.0);
        assert!(!snapshot.has_duration);
        assert!(!snapshot.media_loaded);
    }

    /// Scenario A (cont.): once episode 2's metadata arrives, the slider follows
    /// episode 2, not episode 1.
    #[test]
    fn loaded_media_reports_its_own_duration() {
        let snapshot = resolve_timeline(true, Some(2.0), Some(1440.0));

        assert_eq!(snapshot.position, 2.0);
        assert_eq!(snapshot.duration, 1440.0);
        assert!(snapshot.has_duration);
        assert!(snapshot.media_loaded);
    }

    /// Scenario B: a resume position for the new episode is applied after it
    /// loads; it belongs to the new media and is clamped to its duration.
    #[test]
    fn position_is_clamped_to_known_duration() {
        let snapshot = resolve_timeline(true, Some(9999.0), Some(300.0));

        assert_eq!(snapshot.position, 300.0);
        assert_eq!(snapshot.duration, 300.0);
    }

    /// Scenario D: duration cannot be determined. Never fabricate one, and never
    /// expose an invalid seek range.
    #[test]
    fn unknown_duration_is_not_fabricated() {
        for raw_duration in [None, Some(0.0), Some(-5.0), Some(f64::NAN)] {
            let snapshot = resolve_timeline(true, Some(7.0), raw_duration);

            assert_eq!(snapshot.duration, 0.0, "duration={raw_duration:?}");
            assert!(!snapshot.has_duration);
            // Real position is still reported for the readout.
            assert_eq!(snapshot.position, 7.0);
        }
    }

    #[test]
    fn non_finite_or_negative_position_is_normalised() {
        for raw_position in [None, Some(-1.0), Some(f64::NAN), Some(f64::INFINITY)] {
            let snapshot = resolve_timeline(true, raw_position, Some(600.0));

            assert_eq!(snapshot.position, 0.0, "position={raw_position:?}");
            assert!(snapshot.has_duration);
        }
    }

    #[test]
    fn not_loaded_ignores_every_raw_reading() {
        let snapshot = resolve_timeline(false, Some(1.0), Some(2.0));

        assert_eq!(snapshot, TimelineSnapshot::default());
    }
}

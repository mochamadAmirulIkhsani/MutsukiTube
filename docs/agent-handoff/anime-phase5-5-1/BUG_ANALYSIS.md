# Phase 5.5.1 — Anime UX Refinement — BUG ANALYSIS

Root-cause analysis for the four confirmed real-app defects. Every entry names
the exact mechanism in code, not a symptom.

## B1 — Seek slider keeps the previous episode's values (P0)

**Symptom.** After switching episodes, the slider sometimes shows an old maximum
(e.g. `800` from the previous episode) or an old position until the new file
loads.

**Root cause.** `main.rs` polled libmpv's raw properties every 200 ms:

```rust
let position = session.controller.position().unwrap_or(0.0);
let duration = session.controller.duration().unwrap_or(0.0);
if position.is_finite() && position >= 0.0 { ui.set_playback_position(position as f32); }
if duration.is_finite() && duration >= 0.0 { ui.set_playback_duration(duration as f32); }
```

`EmbeddedMpvPlayer::load()` issues `loadfile replace`. libmpv keeps returning the
**outgoing** file's `duration`/`playback-time` until it has parsed the incoming
file's metadata. The controller only reset the Slint values once, on the single
tick that consumed `pending_anime_source`; the very next tick re-read the stale
libmpv values and wrote them back. The slider maximum is
`playback-duration > 0 ? playback-duration : 1`, so a stale `800` was shown as
the incoming episode's range. Timing-dependent → "sometimes".

**Fix.**
- New pure resolver `apps/desktop/src/player_controller/timeline.rs`
  (`resolve_timeline`) returns an **empty** snapshot while the media is not
  loaded; only finite, in-range values are accepted once loaded; position is
  clamped to a known duration; an absent/non-positive duration is reported as
  *unknown*, never fabricated.
- `EmbeddedPlayerController` gains `media_loaded` + `generation`. `load_media`
  clears `media_loaded`, bumps the generation, and drains queued libmpv events
  (`EmbeddedMpvPlayer::drain_events`) so a late `FileLoaded` for the outgoing
  file cannot mark the incoming media as loaded. `FileLoaded` sets
  `media_loaded = true`; `stop()`/`shutdown()` clear it.
- `main.rs` now consumes `session.controller.timeline()` instead of raw
  properties, and clears the duration to `0` while it is unknown.
- `stop()` already reset state; the new gate makes the reset authoritative.

## B2 — Whole Details page scrolls; episode list is not independent (P1)

**Root cause.** `anime-page.slint` wrapped the entire Details view (hero poster,
text, actions, episodes) in a single `ScrollView`, and the episode rows were a
plain `for` inside it. Scrolling moved the header/hero/actions too.

**Fix.** Details is now a fixed `VerticalLayout`: a fixed-height hero
(`288px`) and an episodes region with `vertical-stretch: 1` whose rows live in a
`ListView`. `ListView` inherits `ScrollView`, so it scrolls and virtualises
internally; nothing outside it moves.

## B3 — Episodes listed newest-first (P1)

**Root cause.** Providers scrape listing pages that often render newest-first
(episode 12 → 1). The host deserialised `manager.episodes()` in provider order
and rendered it as-is.

**Fix.** New `crates/mutsukitube-anime/src/ordering.rs`:
`order_episodes()` stable-sorts ascending by the structured `number` (positive
values), with a conservative title fallback for unnumbered rows. Applied once in
`AnimeManager::episodes()` — the single choke point every consumer uses.
Ordering only changes display order: the same `Episode` values (and therefore the
same `number`/`url`) are moved, so playback, resume and favorites still resolve
the same episode. Unnumbered entries (movies/specials/OVAs) keep stable relative
order and are placed after numbered episodes; they are never assigned a number.

## B4 — Old-fashioned default-widget buttons (P1)

**Root cause.** Details/Home/Watch used `std-widgets` `Button`, which renders
with the style's default chrome and no hierarchy.

**Fix.** New `apps/desktop/ui/components/action-button.slint` defines one
`ActionButtonBase` (height, radius, hover/pressed/focus/disabled states, subtle
motion, real `clicked`) and three variants driven by `theme.slint` tokens:
`PrimaryButton` (Play/Resume), `SecondaryButton`, `ToggleButton` (Favorite).
Details gives Play/Resume visual dominance; Favorite is a toggle; Watch reuses
the same components. No decorative buttons were added, and no callbacks changed.

## Latent defect fixed alongside (progress attribution)

`spawn_playback` set `anime_now_playing` to the **incoming** episode before the
stream URL resolved (seconds of wall-clock time). Meanwhile the 200 ms timer was
still persisting the **outgoing** episode's live position under the incoming
episode's identity. Symmetrically, starting YouTube never cleared the anime
identity, so anime progress could be written from a YouTube video's timeline
(Scenario E).

**Fix.**
- New `AppState.anime_pending_playing`. `spawn_playback` sets it only once the
  stream is resolved, in the same event-loop turn that requests the load (so it
  can never be cleared by a navigation that happens while the stream is still
  resolving).
- `main.rs` is the only writer of `anime_now_playing`: it promotes
  `anime_pending_playing` exactly when it applies the media load, after flushing
  the outgoing episode through the new `flush_anime_progress()` helper.
- Leaving the Watch page flushes and clears the durable identity; starting
  YouTube flushes and clears it too. `anime_pending_*` are left untouched, since
  a just-requested load may still consume them.
- The periodic save / resume-application block is now gated on
  `media_loaded` **and** no pending load, so the transition window cannot leak.

## Constraint compliance

- No fabricated metadata: the hero only renders fields providers actually supply.
- No new decorative buttons; every control keeps its original callback.
- No Electron/WebView/HTML, no media-server backend, no ASUS changes.
- Durable identity stays `provider_id|anime_url` + episode number; stream URLs
  are never persisted or logged in full.

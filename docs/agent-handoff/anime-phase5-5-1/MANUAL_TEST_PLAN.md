# Phase 5.5.1 — Manual Test Plan

Nothing here has been observed in the running app; these steps exist so a human
can move each item from NOT_RUN to verified/failed. Record results in
`TEST_RESULTS.md`.

Launch: `cargo run -p mutsukitube-desktop` (kill any running
`mutsukitube-desktop.exe` first). Plugins come from
`MUTSUKITUBE_ANIME_PLUGIN_DIR` when set, otherwise the installed plugin dir.

## Scenario A — no stale timeline on episode switch (P0)

1. Play an episode long enough for metadata to arrive; note the slider maximum.
2. Go back to Details and play a **different** episode.
3. Watch the slider from the moment the new episode is requested until playback
   starts.

Pass: the maximum never shows the previous episode's duration; it is `0`/
disabled while loading, then becomes the new episode's duration. The position
never jumps to the old episode's value.

## Scenario B — resume correctness

1. Watch an episode past ~30 s, then leave (Details or another section).
2. Return to the same anime; the episode row / Play button offers resume.
3. Resume it.

Pass: playback starts at the saved position (±a few seconds), is clamped to the
episode duration, and does **not** seek twice.

## Scenario C — rapid switching ignores stale events (P0)

1. Play episode 1; as soon as it starts, immediately play 2, then 3.
2. Watch the slider and the title/label throughout.

Pass: only episode 3's timeline is ever shown; no flicker back to 1 or 2; the
episode label matches the media that actually plays.

## Scenario D — unknown duration

1. Play an item whose duration is not yet known (fresh load, slow source).

Pass: the slider is disabled and shows no fabricated maximum; the position
readout does not invent a duration. It becomes usable once real metadata arrives.

## Scenario E — anime → YouTube, no leakage

1. Watch an anime episode for ~40 s.
2. Play a YouTube video.
3. Return to the anime section; check Continue Watching.

Pass: the YouTube video's position/duration is never written to the anime
episode; the anime's preserved position is the one from step 1.

## Scenario F — seek + persistence across restart

1. Watch an episode; drag the slider to a new position and release.
2. Wait ~6 s (persist interval), then close the app.
3. Relaunch and resume the same episode.

Pass: the resumed position reflects the seek (not the pre-seek position); no
duplicate/uncommitted seek; the dragged position was not yanked mid-drag.

## Layout — independent episode scrolling (P1)

For **1366×768** and **1920×1080**:

1. Open an anime with many episodes (≥ 24; ideally ≥ 100 if available).
2. Mouse-wheel over the episode list.
3. Open the Windows "maximize"/resize to the smallest allowed window (min is
   800×600).

Pass: only the episode list scrolls; the header, hero (poster/title/actions) and
the "Episodes" heading stay fixed; no outer page scroll; rows do not overlap the
action buttons; the list stays usable at the minimum window size.

## Ordering (P1)

1. Open an anime whose provider lists newest-first.

Pass: rows read Episode 1, 2, 3 … ascending; clicking row *N* plays episode *N*
(verify the label); previously-saved resume for an episode still maps to the
same episode.

## Buttons (P1/P2)

1. Hover, press, Tab to, and activate Back / Search / Play-Resume / Favorite /
   quality selector.

Pass: hover/pressed feedback is visible and minimal; keyboard focus is visibly
indicated; Enter/Space activate the focused control; disabled controls are
visibly dimmed and inert; Favorite reads as selected when on; nothing looks like
a beveled/default desktop button; there are no buttons that do nothing.

## Regression — YouTube and playback (must not break)

1. YouTube: search, play, pause, seek, stop, replay, fullscreen, favorite,
   playlists, history, navigation.
2. Anime: window resize, Continue Watching, install/rollback/offline status.

Pass: all unchanged from Phase 5.5.

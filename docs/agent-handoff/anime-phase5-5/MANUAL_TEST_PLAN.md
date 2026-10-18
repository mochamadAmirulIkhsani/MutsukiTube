# Phase 5.5 — Manual Test Plan (Anime)

Run these by hand on the Windows desktop. Record actual observations; do not
assume. None of this was observed by the agent.

## 0. Build & launch

```powershell
cd D:\Projects\mutsukitube-folder\mutsukitube
cargo build -p mutsukitube-desktop
.\target\debug\mutsukitube-desktop.exe
```

Optional debug logs: set `$env:RUST_LOG="debug"` or `$env:MUTSUKITUBE_DEBUG=1`.

## 1. Plugin onboarding (no plugins installed)

1. Temporarily point the app at an empty plugin dir:
   `$env:MUTSUKITUBE_ANIME_PLUGIN_DIR="$env:TEMP\empty-plugins"` (create it).
2. Launch and open **Anime**.
3. Expect: setup screen — "Anime plugins are required to search and watch
   anime." + **Install Anime Plugins**. No broken empty library.
4. Without `MUTSUKITUBE_PLUGIN_REGISTRY_URL`, installing reports "Plugin registry
   URL is not configured". With an unreachable URL, expect "Registry offline or
   unreachable".

Record: [ ] setup shown  [ ] no broken grid  [ ] offline message correct

## 2. Home with plugins

1. Point `MUTSUKITUBE_ANIME_PLUGIN_DIR` at a dir containing real signed plugin
   binaries (e.g. `mutsukitube-anime-plugins\dist\v0.1.0`).
2. Open **Anime**.
3. Expect: `home` state. With no history/favorites, an "Find something to
   watch" card. With history, shelves render.

Record: [ ] home shown  [ ] empty state legible  [ ] shelves render when data exists

## 3. Search

1. Type a known title, press Enter (and also click Search).
2. Expect: "Searching for …", then poster cards under "Results for …".
3. Poster images load (or a clean title fallback when a provider has no image).
4. Long titles truncate to 2 lines without overlapping the next row.
5. Search an empty/garbage string: expect no crash; empty/again state.

Record: [ ] results posters  [ ] fallback ok  [ ] truncation ok  [ ] no crash

## 4. Details

1. Click a result card.
2. Expect: details view — poster, title, year badge (only if present), episode
   count, Play, Favorite.
3. Expect: "Episodes" list with landscape rows; episode number visible even with
   no thumbnail.
4. Toggle **Favorite** → expect "Added to My Anime".

Record: [ ] details hero  [ ] episodes load  [ ] number visible without art
[ ] favorite toggles

## 5. Playback + resume

1. From details, click an episode row, then **Play**.
2. Expect: "Finding a playable source…" then switch to the Watch page and
   embedded libmpv starts.
3. Pause/Resume, Seek, Volume, Fullscreen, Back to details.
4. Let it play past ~30s, then Back to details.
5. Expect: that episode shows a "Resume m:ss" status; hero shows **Resume …**.
6. Click **Resume** → expect playback starts and seeks to the saved position
   (log: "Resumed anime at Ns" when debug logging is enabled).
7. Return to **Anime** home → expect the title in **Continue Watching** with a
   progress bar.

Record: [ ] playback starts  [ ] controls work  [ ] resume card appears
[ ] resume seeks correctly  [ ] Continue Watching populated

## 6. History / favorites persistence

1. Watch enough of an episode to pass 90% of a known duration.
2. Expect: it leaves Continue Watching but stays in **Recently Watched**.
3. **My Anime** lists favorites; toggling off removes it.
4. Restart the app → expect history/favorites/Continue Watching still present.

Record: [ ] completed leaves Continue Watching  [ ] persists across restart

## 7. Responsiveness

Resize to 1366×768 and 1920×1080, and test Windows scaling 100% / 125% / 150%.

Record: [ ] poster columns reflow  [ ] no clipped text  [ ] episode rows readable
[ ] details hero does not overlap the player area

## 8. YouTube regression

1. Search YouTube, open a video, play, pause, seek, fullscreen.
2. Expect: unchanged from before Phase 5.5.

Record: [ ] YouTube unaffected

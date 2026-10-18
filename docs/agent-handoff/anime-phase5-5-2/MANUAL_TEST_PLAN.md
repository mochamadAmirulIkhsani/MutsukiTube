# Phase 5.5.2 — Smart Back Navigation & YouTube Home — MANUAL_TEST_PLAN

Nothing here has been observed in the running app; these steps exist so a human
can move each item from NOT_RUN to verified/failed. Record results in
`TEST_RESULTS.md`.

Launch: `cargo run -p mutsukitube-desktop` (kill any running
`mutsukitube-desktop.exe` first).

## Navigation — Requirement A (Back returns to origin)

For each entry point, open a video, then press Back on WatchPage.

1. **YouTube Home → video → Back** → expect YouTube Home (cached feed).
2. **YouTube Search (query "naruto") → video → Back** → expect Search Results
   with query "naruto" still in the box and results visible (no re-search).
3. **YouTube History → video → Back** → expect History.
4. **YouTube Favorites → video → Back** → expect Favorites.
5. **YouTube Playlist → video → Back** → expect the same playlist.
6. **Anime Details → episode → Back** → expect Anime Details for that anime
   (selected anime + episode preserved).
7. **Multiple nested Back actions** → expect stepping back one screen at a
   time, never skipping.
8. **No duplicate entries** → reopening the same screen does not grow the stack
   without bound.
9. **No cross-context navigation** → Back from Anime Details stays in Anime;
   Back from YouTube screens stays in YouTube.
10. **Direct Watch without prior route** → Back returns to YouTube Home
    (safe default).
11. **Rapid repeated Back clicks** → expect one screen per click, no crash.
12. **No stale async callback reopening WatchPage** → after navigating away
    from WatchPage, no background callback re-opens it.

## YouTube Home — Requirement B

1. **Home loads real video metadata** → Explore grid shows real videos from the
   provider (thumbnails, titles, channels).
2. **Topic chips work** → clicking a chip changes the Explore content to that
   topic's real results.
3. **Video cards open correct videos** → clicking a card opens that video in
   WatchPage.
4. **Home results cached** → leaving Home (e.g. to watch a video) and pressing
   Back shows the same feed without re-fetching.
5. **Search results remain intact** → search → video → Back restores the search
   page with query and results.
6. **Empty history does not break Home** → with no watch history, Home still
   shows Explore/chips and is usable.
7. **Offline state degrades gracefully** → if discovery fails, an error/empty
   state is shown with a working Retry; not treated as a network error when
   history is simply empty.
8. **Pagination bounded** → Explore limited to 24 videos per topic; no
   uncontrolled burst of searches on startup.
9. **No fabricated recommendations** → Explore section is labeled as generic
   topic results, not personalized.

## Regression — existing features must still work

- YouTube search, Load More, History, Favorites, Playlists.
- Native/yt-dlp provider modes, embedded libmpv playback.
- Anime search, Anime Details, episode ordering, independent episode scrolling,
  Continue Watching.
- Plugin updater.
- Window resizing.

## NOT_RUN

Visual/interactive verification is NOT possible in this environment (the window
was never observed). All UI checks above are NOT_RUN until a human runs them.

## Playback slider (deferred)

The playback slider bug (stale duration on episode switch) remains DEFERRED and
unfixed. Do not mark it resolved.

# Phase 5.5.2 — Smart Back Navigation & YouTube Home — NAVIGATION_DESIGN

## Root cause of hardcoded Back

Every "Back" handler in `apps/desktop/ui/app.slint` hardcodes the destination:

- WatchPage back: `root.current-page = "search";` (line ~334)
- HistoryPage back: `root.current-page = "search";`
- SettingsPage back: `root.current-page = "search";`
- LibraryPage back: `root.current-page = "search";`

WatchPage has no knowledge of the screen that opened the video. There is no
back stack, no stored origin, and no preserved per-page context (search query,
results, anime selection). Pressing Back always returns to the search page,
which is wrong when the user came from History, Favorites, Playlists, or Anime
Details.

## Navigation stack architecture

A typed navigation model lives in `apps/desktop/src/navigation.rs` (new) and is
stored in `AppState.nav_stack: Vec<NavEntry>` (bounded to 50 entries).

```
NavRoute:
  YouTubeHome
  YouTubeSearch { query }
  YouTubeHistory
  YouTubeFavorites
  YouTubePlaylist { name }
  AnimeHome
  AnimeDetails { key }
  WatchPage { video_id }
  Settings
```

Rules:
- Every meaningful navigation pushes a `NavEntry` (route + human title).
- Consecutive duplicate entries (same route + same context) are not pushed.
- The stack is capped at 50; older entries are dropped from the bottom.
- Root = YouTubeHome, which has no prior route.
- Back pops exactly the current entry and restores the new top (the origin).
- Back never redirects unconditionally to Home and never creates loops.
- Direct WatchPage entry without a valid origin falls back to YouTubeHome.
- Cross-context jumps (e.g. Anime → YouTube) push a fresh entry; Back returns
  within the same context stack because entries carry their own route.

## Screens with correct Back restoration

| From → → WatchPage → Back | Expected restoration |
|---|---|
| YouTubeHome → video | YouTubeHome (fresh/explore feed, cached) |
| YouTubeSearch (query "n") → video | YouTubeSearch: same query + cached results, no re-search |
| YouTubeHistory → video | YouTubeHistory |
| YouTubeFavorites → video | Favorites (library mode=favorites) |
| YouTubePlaylist → video | Same playlist (library mode=playlist) |
| AnimeDetails → episode | Anime Details (anime-view=details, selected anime+episode preserved) |
| Direct watch (no origin) | YouTubeHome (safe default) |

State restored separately from screen names:
- Search: `current_query` + `search_results` persist in `AppState`; the query is
  echoed back into the search box; no automatic re-search when results are
  still valid.
- Anime: `anime_view`, `selected_anime`, `selected_episode`,
  `anime_selected_episode_number`, `anime_selected_key` persist.
- History/Favorites/Playlist: read from durable SQLite storage on entry.

## What is NOT stored

Temporary signed stream URLs / libmpv media sources are never placed in the
navigation stack. Only stable identities (video id, anime key, query, playlist
name).

## Anime boundary

Anime intra-app navigation (home ↔ details) uses the existing `anime_view`
string and is untouched. Anime Details → WatchPage pushes a WatchPage entry
whose origin is AnimeDetails; Back restores the anime details view. YouTube and
Anime do not misroute into one another because each entry carries its own route.

## Keyboard

Escape (fullscreen exit) and existing media shortcuts are unchanged. No new
navigation shortcuts are added that could interfere with typing or player
controls.

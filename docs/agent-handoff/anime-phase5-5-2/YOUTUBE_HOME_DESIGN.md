# Phase 5.5.2 — Smart Back Navigation & YouTube Home — YOUTUBE_HOME_DESIGN

## Goals

- Replace the near-empty YouTube start page with a browsable, content-rich home.
- Use only existing providers and local storage — no login, no API key, no
  cookies, no fabricated recommendations.
- Keep MutsukiTube's own visual identity (dark media-library theme from
  Phase 5.5.1).

## Data sources (in priority order)

1. **Continue Watching / Recently Watched** — local SQLite watch history
   (`mutsukitube-storage::get_history`), rendered as a shelf.
2. **Favorites** — local SQLite favorites (`get_favorites`), rendered as a
   shelf.
3. **Explore** — real provider search results for generic topics
   (`Music`, `Gaming`, `Technology`, `Podcasts`, `Education`) via the existing
   `YoutubeProvider` (native InnerTube + yt-dlp fallback). Bounded to 24
   videos per topic, one search per topic change, generation-cached.
4. **Offline fallback** — the empty state steers the user to pick a topic or
   search; cached history/favorites are shown when present.

Generic topic results are explicitly labeled "Explore" — never presented as
personalized recommendations. No official YouTube Data API is used.

## UI structure (`ui/pages/home-page.slint`)

- Header: MutsukiTube title, search field, Search, History, Favorites, Anime.
- Topic chip row: All, Music, Gaming, Technology, Podcasts, Education — real
  click handlers that load the corresponding explore topic.
- Continue Watching shelf (horizontal, 16:9 cards) when history exists.
- Favorites shelf when favorites exist.
- Explore grid of real video cards (thumbnail, title, channel, duration).
- Empty state with "Browse Topics" when nothing is available yet.
- Status bar for loading/error messages.

Video cards reuse the Phase 5.5 `VideoCard` component. Buttons reuse the
Phase 5.5.1 `PrimaryButton`/`SecondaryButton`/`ToggleButton` action-button
components with hover/pressed/focus/disabled states.

## Caching strategy

- Explore results are kept in the Slint `home-videos` model, so returning to
  Home from WatchPage shows the cached feed without re-fetching.
- `home_generation` in `AppState` prevents stale async results from
  overwriting newer page state.
- History/favorites are read from durable SQLite on home entry.

## Navigation integration

- Home is the root route (`youtube-home`), with no prior route.
- Search from Home pushes `YouTubeSearch { query }` and shows search results.
- Back from Search returns to Home (cached feed preserved).
- Back from WatchPage returns to Home (or the screen that opened the video).

# Phase 5.5 — Anime UX Audit

Scope: the Anime section of MutsukiTube. This audit records what existed before the
Phase 5.5 overhaul, what data is actually available, and where the implementation
had to stop short of the Jellyfin-inspired target.

## Before (pre-5.5)

`apps/desktop/ui/pages/anime-page.slint` was a single vertical stack:

- A search `LineEdit` + `Button`.
- A flat `[string]` list of search result titles (no posters, no cards).
- A flat `[string]` list of episode titles.
- A `ComboBox` of qualities.
- Play / Back / Install Plugins buttons.
- Status text.

`apps/desktop/src/handlers/anime.rs` did:

- `search_auto` → mapped `Anime` results to `title` strings only.
- `select-anime(int)` by list index, then `episodes(...)`.
- `select-episode(int)` by list index, then `heights(...)`.
- `play()` → `stream_with_fallback`, then `anime_play_ready`.
- Install Plugins flow (signed manifest, verify, install, activate).

There was no poster rendering, no home/library, no history, no favorites, no
resume, no Continue Watching, and no plugin-onboarding screen.

## Data actually available (constraints)

`crates/mutsukitube-anime/src/models.rs`:

| Model     | Fields                                                                 |
|-----------|------------------------------------------------------------------------|
| `Anime`   | `provider_id`, `title`, `url`, `thumbnail_url?`, `alt_titles[]`, `year?`|
| `Episode` | `provider_id`, `number`, `title`, `url`                                |
| `StreamInfo` | `url`, `referer?`, `headers`, `height?`                             |

Providers expose **no** synopsis, genres, studios, ratings, backdrops, season
groupings, or separate episode artwork. Continue Watching / favorites did not
exist in `crates/mutsukitube-storage`.

Consequence: this overhaul renders only real fields. It does **not** fabricate
metadata. Sections that require missing data are omitted rather than faked.

## After (Phase 5.5)

- Design tokens in `apps/desktop/ui/theme.slint`.
- Reusable components:
  - `components/anime-poster-card.slint` (`AnimeCard` struct, 2:3 poster, hover/
    selected states, progress bar, badge, image fallback).
  - `components/episode-row.slint` (`EpisodeItem` struct, landscape thumbnail,
    episode number always visible, watched/resume status).
  - `components/poster-shelf.slint` (heading + wrapping `FlexboxLayout` grid).
- `pages/anime-page.slint` rebuilt around three states — `setup`, `details`,
  `home` — with a fixed header (title, search, Back).
- Home shelves: Search Results, Continue Watching, Recently Watched, My Anime
  (favorites), plus an empty onboarding state.
- Details: hero with poster, title, year badge when present, episode count,
  Play/Resume, Favorite toggle, optional quality ComboBox, episode browser.
- Storage: `crates/mutsukitube-storage/src/anime.rs` adds `anime_history`,
  `anime_progress`, `anime_favorites` tables with tested read/write helpers.
- `apps/desktop/src/main.rs` persists progress every ~5s while playing and
  applies a resume seek once playback starts.

## Gaps vs the Jellyfin-inspired target (honest)

| Target | Status | Reason |
|---|---|---|
| Poster grids | Done | Uses provider `thumbnail_url`. |
| Backdrop/hero image | Poster-only fallback | Providers supply no backdrop. |
| Synopsis / genres / studio / rating | Not shown | Not provided; not fabricated. |
| Release year | Shown when `year` present | Often absent upstream. |
| Season navigation | Not shown | Source is a flat episode list. |
| Episode thumbnails | Neutral EP-number fallback | Not provided. |
| Episode duration | Shown when a stored duration exists | Only after a play session. |
| Next/Previous episode in player | Not implemented | No fake buttons added. |
| Catalog/trending/seasonal browse | Not shown | Providers are search-first. |
| Virtualized long episode lists | `ScrollView` + `for` | Acceptable for typical counts. |

## Not visually verified

The native Slint window was **not** observed on screen in this environment. All
claims about layout are "compiles and is wired to real state", not "pixels
confirmed". See `MANUAL_TEST_PLAN.md` for the exact steps a human must run.

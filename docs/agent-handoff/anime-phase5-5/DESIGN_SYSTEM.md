# Phase 5.5 — Anime Design System

## Principles

- Dark media-library surface, poster-driven, minimal chrome.
- Only real data is shown. Missing metadata is omitted, never invented.
- Reusable Slint tokens/components; no per-screen magic values.
- Lightweight: no animation beyond hover/selection, no heavy shadows.
- The YouTube section is unchanged; this system targets the Anime section only.

## Tokens — `apps/desktop/ui/theme.slint`

`Theme` global (read-only properties):

- Surfaces: `bg #0e0f12`, `surface #16181d`, `surface-hover #1f232b`,
  `surface-elevated #1a1d24`, `surface-sunken #101216`.
- Accent: `accent #7c5cff`, `accent-hover #9077ff`, `accent-soft #2a2350`.
- Text: `text-primary #f3f4f6`, `text-secondary #a8adb8`, `text-muted #6f7683`.
- Lines: `border #262a33`, `border-strong #3a404c`.
- Status: `success #43c59e`, `danger #e5645f`.
- Geometry: `radius 10px`, `radius-sm 6px`, `poster-width 148px`,
  `poster-height 222px` (2:3), `gap 16px`.

## Components

### `AnimePosterCard` (`components/anime-poster-card.slint`)

- `AnimeCard` struct: `key`, `provider-id`, `url`, `title`, `subtitle`, `badge`,
  `progress`, `has-poster`, `poster`.
- Portrait poster clipped to `radius`, `image-fit: cover` (no distortion).
- Hover/selected states via `TouchArea.has-hover` and an accent border.
- Optional badge (e.g. `EP 7`) and Continue Watching progress bar — drawn only
  when the value is real.
- Title `max-lines: 2` with elide; optional one-line subtitle.
- Fallback: centered title when `has-poster` is false.

### `EpisodeRow` (`components/episode-row.slint`)

- `EpisodeItem` struct: `number`, `title`, `status`, `duration`, `watched`,
  `playing`, `has-thumbnail`, `thumbnail`.
- Landscape 128×66 thumbnail; neutral `EP <n>` fallback keeps episode numbers
  visible without artwork.
- Watched check, resume status, and duration render only when known.
- Selected row uses `accent-soft`; hover uses `surface-hover`.

### `PosterShelf` (`components/poster-shelf.slint`)

- Heading + `FlexboxLayout` (`flex-wrap: wrap`) of `AnimePosterCard`.
- Wrapping gives responsive columns from window width with no manual column math.

## Page composition — `pages/anime-page.slint`

- Header (64px, sunken): Back, `Anime` title, search field, Search button.
- Status bar (shown only when loading or status text exists).
- Three sibling views, each behind a mutually-exclusive `if root.view == "…"`
  conditional with `vertical-stretch: 1`, so exactly one is in the layout
  (`setup`, `details`, `home`). Slint's `visible` property does not remove an
  element from layout, so conditionals are used instead.
  - `setup`: onboarding panel + Install Anime Plugins.
  - `details`: hero (poster + title + badge + episode count + Play/Resume +
    Favorite + optional quality) then episode rows.
  - `home`: Search Results / Continue Watching / Recently Watched / My Anime
    shelves, or an empty onboarding card when all are empty.

## State flow

`app.slint` holds the view models; `handlers/anime.rs` owns the logic:

- `anime-view`: `setup` | `details` | `home`.
- `anime-plugins-ready`: any provider executable present.
- Card models are `[AnimeCard]`; episode model is `[EpisodeItem]`.
- Card `key` = `provider_id|anime_url` (stable identity, never a stream URL).

## Why no backdrop

Providers return no backdrop image. The hero therefore uses the portrait poster
plus a sunken panel rather than blurring/upscaling a low-res poster, which the
spec explicitly forbids.

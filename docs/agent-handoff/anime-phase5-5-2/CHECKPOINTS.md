# Phase 5.5.2 — Checkpoints

Statuses: `implemented` · `tested` (automated) · `verified` (observed in app) ·
`deployed` (shipped). No checkpoint is `verified` in this environment.

## NAV-00 — Code Audit & Existing Navigation Investigation

- Audited `app.slint`, all pages (`watch-page`, `search-page`, `history-page`,
  `library-page`, `settings-page`, `anime-page`), handlers
  (`search`, `history`, `library`, `player`, `player_controls`, `settings`,
  `anime`), `state.rs`, `main.rs`.
- Documented current behavior and root cause of hardcoded Back in
  `NAVIGATION_DESIGN.md`.
- Baseline: `cargo check --workspace`, `cargo test --workspace`,
  `cargo build -p mutsukitube-desktop` all green on Phase 5.5.1 tip `b7bf8c6`.
- Status: **tested** (baseline green).

## NAV-01 — Typed Navigation State & Back Stack

- `apps/desktop/src/navigation.rs` (new): `NavRoute`/`NavEntry`, bounded stack
  (50 entries), no consecutive duplicates, `push_entry`/`pop_current`,
  `restore_destination`, `activate_route` (loads history/library on return),
  `push_navigation`/`navigate_back`.
- `AppState.nav_stack: Vec<crate::navigation::NavEntry>` seeded with root
  YouTubeHome; `current-page` default changed to `"youtube-home"`.
- main.rs: `nav-back`, `nav-search`, `nav-history`, `nav-favorites`,
  `nav-library-playlist`, `nav-settings`, `nav-anime`, `nav-anime-details`,
  `nav-home`, `load-home`, `load-home-topic` callbacks registered.
- player.rs: `open-video` pushes a WatchPage entry instead of hardcoding the
  page.
- anime.rs: `anime-play-ready` pushes a WatchPage entry; `anime-select`
  pushes an AnimeDetails entry; obsolete `on_anime_back` removed.
- Status: **implemented**.

## NAV-02 — WatchPage Back Behavior & State Restoration

- WatchPage/History/Settings/Library/Anime back handlers all call
  `root.nav-back()`, which pops the current stack entry and restores the
  origin (search query echoed, results not re-fetched; anime details view
  restored; history/favorites/library re-activated from storage).
- Direct WatchPage entry without a valid origin falls back to YouTubeHome.
- Status: **implemented**.

## HOME-03 — YouTube Home Data Sources & Feed Architecture

- `apps/desktop/src/handlers/home.rs` (new): `load_home` loads Continue
  Watching (history), Favorites, and a bounded default Explore topic
  ("Music") via the existing `YoutubeProvider` (native InnerTube + yt-dlp
  fallback). `load_topic` switches the Explore topic on chip click.
- Bounded to `EXPLORE_LIMIT` (24) per topic; one search per topic change;
  generation-based staleness prevents stale async overwrites.
- No login, no API key, no cookies; generic topic results labeled "Explore",
  never personalized recommendations.
- Status: **implemented**.

## HOME-04 — Modern YouTube Home UI

- `apps/desktop/ui/pages/home-page.slint` (new): header with search +
  navigation buttons, topic chip row (All/Music/Gaming/Technology/Podcasts/
  Education), Continue Watching shelf, Favorites shelf, Explore grid, empty
  state. Uses theme tokens and the Phase 5.5.1 action-button components.
- `ui/app.slint`: new `youtube-home` route rendering HomePage; search page is
  now search-results-only.
- Status: **implemented** (visual verification NOT_RUN).

## HOME-05 — History, Favorites, Search & Category Integration (in progress)

- Search query/results survive return from WatchPage (no re-search).
- Topic chips trigger real explore searches.
- History/Favorites/Playlist restore via `activate_route`.
- Status: **implemented**.

## QA-06 — Automated Testing & Regression

- `cargo fmt --all -- --check` (clean), `cargo check --workspace` (no warnings),
  `cargo test --workspace` (all green), `cargo build -p mutsukitube-desktop`.
- Added 6 navigation-stack unit tests in
  `apps/desktop/src/navigation.rs` (desktop unit tests 6 → 12).
- Regression confirmed: existing YouTube search/load-more/history/favorites/
  playlists, native/yt-dlp, embedded libmpv, Anime search/details, episode
  ordering, independent scrolling, Continue Watching, plugin updater all still
  pass.
- Slider bug remains deferred (not fixed, not marked fixed).
- Status: **tested**.

## QA-07 — Final UX Handoff

- Wrote `STATE.md`, `CHECKPOINTS.md`, `TEST_RESULTS.md`,
  `NAVIGATION_DESIGN.md`, `YOUTUBE_HOME_DESIGN.md`, `MANUAL_TEST_PLAN.md`.
- Local commit `aca2b8d` made.
- Visual/manual acceptance **NOT_RUN** (window never observed).


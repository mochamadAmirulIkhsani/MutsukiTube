# Phase 5.5 — Checkpoints

Status legend: **Implemented** (code exists), **Tested** (automated tests pass),
**Verified** (observed working end-to-end by a human), **Deferred**.

No checkpoint is marked Verified: the native window was not observed in this
environment. See `MANUAL_TEST_PLAN.md`.

---

## UX-01 Design System

- **Implemented**: `ui/theme.slint` tokens; `AnimePosterCard`, `EpisodeRow`,
  `PosterShelf` components; `anime-page.slint` rebuilt. See `DESIGN_SYSTEM.md`.
- **Tested**: workspace compiles; Slint UI compiles with no warnings.
- Evidence: `apps/desktop/ui/theme.slint`,
  `apps/desktop/ui/components/*.slint`.

## UX-02 Navigation

- **Implemented**: header with Back; `anime-view` state machine
  (`home`/`details`/`setup`). Back from details → home (reloads shelves); Back
  from home → Search page. Search preserves results in state while navigating to
  details and back.
- **Tested**: compiles; handler logic in `handlers/anime.rs` (`on_anime_back`).
- **Deferred**: scroll-position restoration on back is not persisted.

## UX-03 Search and Cards

- **Implemented**: persistent search field in the header; Enter and button both
  search; stale-result protection via `anime_generation`; empty/loading/error
  states; results render as poster cards in the Results shelf. Provider fallback
  and priority unchanged (no mirror selector exposed).
- **Tested**: generation-guard logic present; compiles.
- Evidence: `handlers/anime.rs::register_search`.

## UX-04 Anime Discovery, Details & Episodes

- **Implemented**: `home` shelves from local storage (Continue Watching,
  Recently Watched, My Anime), empty onboarding card, Jellyfin-style details hero
  (poster, title, year badge when present, episode count, Play/Resume, Favorite),
  and a landscape episode browser. Flat episode list (no fake seasons).
- **Not fabricated**: no synopsis/genres/studio/rating/backdrop — unavailable.
- **Tested**: `available_provider_ids` unit/integration tests; storage round
  trips; compiles.

## UX-05 WatchPage

- **Implemented**: existing embedded libmpv integration preserved. Detail →
  Play/Resume resolves the stream via `stream_with_fallback`, switches to the
  Watch page, and the player starts. Resume position is seeked once playback
  begins. Progress is persisted every ~5s.
- **Tested**: existing embedded-player tests pass (2). Playback itself not
  observed here (no anime episode streamed in this environment).
- **Deferred**: Next/Previous Episode buttons — not added rather than faked.

## UX-06 Plugin Installation

- **Implemented**: when no provider executable is present, Anime shows the
  `setup` onboarding ("Anime plugins are required to search and watch anime.")
  with a primary Install Anime Plugins action. Install flow reports
  checking/downloading/verifying/installing/ready, "Registry offline or
  unreachable" for network errors, and the known-bad reinstall guard. After
  success the registry is reloaded so Home becomes available.
- **Not claimed**: the ASUS registry is not deployed. Signed local fixtures
  remain the test path.
- **Tested**: updater unit (13) + Go-manifest compat (4) tests pass.

## UX-07 History and Favorites

- **Implemented**: new SQLite tables `anime_history`, `anime_progress`,
  `anime_favorites` with helpers in `crates/mutsukitube-storage/src/anime.rs`.
  Continue Watching uses real local progress; favorites toggle from details.
  Watched threshold: `WATCHED_COMPLETION_THRESHOLD = 0.9` of known duration.
  Resume requires `MIN_RESUME_SECONDS = 30` watched.
- **Tested**: 8 storage unit tests over an in-memory DB (progress upsert,
  per-episode scoping, completion → history, minimum threshold, favorite toggle).
- **Not fabricated**: progress is written only from real playback positions.

## UX-08 Responsive Layout

- **Implemented**: `FlexboxLayout` wrapping for poster grids; `ScrollView` for
  content; fixed-width hero poster + flexible text column; episode rows use a
  fixed landscape thumbnail and flexible text.
- **Tested**: compiles. Not observed at 1366×768 / 1920×1080 / 100–150% scaling.

## UX-09 End-to-End Testing

- **Implemented**: `cargo fmt --check`, `cargo check --workspace`,
  `cargo test --workspace`, `cargo build -p mutsukitube-desktop` all pass.
- **Tested**: see `TEST_RESULTS.md`.
- **Deferred (human)**: manual anime playback, visual inspection, plugin install
  against a real signed registry. See `MANUAL_TEST_PLAN.md`.

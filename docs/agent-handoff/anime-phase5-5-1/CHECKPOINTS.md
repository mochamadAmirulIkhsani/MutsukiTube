# Phase 5.5.1 — Checkpoints

Each checkpoint: what changed, how it was tested, current status. Statuses:
`implemented` (code written) · `tested` (automated tests green) ·
`verified` (observed in the running app) · `deployed` (shipped). No checkpoint is
`verified` in this environment — the window was never observed.

## REF-00 — Audit and reproduce

- Read `player_controller/mod.rs`, `main.rs`, `watch-page.slint`,
  `handlers/{anime,player,player_controls}.rs`, `anime-page.slint`,
  `theme.slint`, components, `mutsukitube-anime` manager/models/ordering inputs,
  `mutsukitube-storage/src/anime.rs`, `mutsukitube-embedded-player/src/lib.rs`.
- Confirmed all four mechanisms (see `BUG_ANALYSIS.md`) plus the latent
  progress-attribution defect. Branch `fix/anime-ux-refinement` from `5d5126b`.
- Status: **verified by code reading** (root causes pinned, not guessed).

## REF-01 — Timeline isolation (P0 / B1)

- `player_controller/timeline.rs`: `TimelineSnapshot` + `resolve_timeline`.
- `EmbeddedMpvPlayer::drain_events()`.
- `EmbeddedPlayerController`: `media_loaded`, `generation`, `media_loaded()`,
  `media_generation()`, `timeline()`; cleared on load/stop/shutdown; set on
  `FileLoaded`.
- `main.rs`: use `timeline()`; clear duration when unknown; skip position push
  while seeking.
- `watch-page.slint`: `seeking` set by `changed`/`released`; `app.slint` exposes
  `timeline-seek-active`.
- Tests: 6 timeline unit tests (Scenarios A, B, D + normalisation).
- Status: **tested** (visual feel NOT_RUN).

## REF-02 — Independent episode scrolling (P1 / B2)

- `anime-page.slint`: Details restructured to fixed hero (`288px`) + episodes
  region (`vertical-stretch: 1`) containing a `ListView`.
- Status: **tested** (compiles; layout NOT_RUN — needs a human).

## REF-03 — Ascending episode ordering (P1 / B3)

- `crates/mutsukitube-anime/src/ordering.rs`: `order_episodes`,
  `parse_episode_number`; exported from `lib.rs`; called in
  `AnimeManager::episodes()`.
- Fake provider `many-episodes` mode returns 12,11,10,2,1.
- Tests: 8 unit tests + integration `episodes_are_returned_in_ascending_order`
  (asserts numbers `[1,2,10,11,12]` and URL/identity preservation).
- Status: **tested**.

## REF-04 — Modern Details controls (P1 / B4)

- `ui/components/action-button.slint` + theme tokens
  (`control-height`, `control-radius`).
- Applied to Anime header (Back/Search), setup (Install), details
  (Play/Resume primary, Favorite toggle, quality combo), and Watch transport.
- Status: **tested** (states NOT_RUN — needs a human).

## REF-05 — Home / visual polish (P2)

- Card hover border (`Theme.border-strong`); shelf heading accent marker.
- Status: **tested** (look NOT_RUN).

## REF-06 — Regression + manual QA handoff (P2)

- `cargo fmt --all -- --check` (clean), `cargo check --workspace` (no warnings),
  `cargo test --workspace` (all green), `cargo build -p mutsukitube-desktop`.
- Fixed three pre-existing warnings in `crates/mutsukitube-anime/tests/integration.rs`.
- Wrote `BUG_ANALYSIS.md`, `TEST_RESULTS.md`, `MANUAL_TEST_PLAN.md`, this file,
  `STATE.md`. Local commit.
- Status: **tested**; visual/manual acceptance **NOT_RUN**.

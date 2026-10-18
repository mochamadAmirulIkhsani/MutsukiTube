# Phase 5.5.1 — Anime UX Refinement — STATE

Authoritative resume state. Keep this file current.

## Repository

- Rust repo: `D:\Projects\mutsukitube-folder\mutsukitube`
- Branch: `fix/anime-ux-refinement` (local only — **not pushed**)
- Base: `5d5126b` (`feat/anime-ux-phase5-5` tip)
- Code + docs commit: see `CHECKPOINTS.md` REF-06
- Go repo: `D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins` — untouched

## Objective

Fix the four confirmed real-app defects on top of Phase 5.5 and modernise the
Anime UI, preserving YouTube, embedded libmpv, `AnimeManager`, the Go IPC
providers, and the signed updater.

Priority order actually followed: P0 slider → P1 episode scrolling → P1 numeric
ordering → P1 buttons → P2 polish + regression tests.

## Done

- **B1 (P0, slider).** Pure `resolve_timeline` in
  `apps/desktop/src/player_controller/timeline.rs`; `EmbeddedPlayerController`
  gained `media_loaded`/`generation` + `drain_events()`; `main.rs` consumes
  `controller.timeline()`; duration cleared while unknown; position update
  suppressed while the user drags (`watch-page.seeking` → `app.timeline-seek-active`).
- **B2 (P1, scrolling).** Details = fixed hero + `ListView` episodes
  (`vertical-stretch: 1`); no outer `ScrollView` on Details.
- **B3 (P1, ordering).** `crates/mutsukitube-anime/src/ordering.rs`
  (`order_episodes`, `parse_episode_number`), applied in `AnimeManager::episodes()`;
  decimal/year/resolution safety; 8 unit tests + 1 integration test.
- **B4 (P1, buttons).** `ui/components/action-button.slint`
  (`PrimaryButton`/`SecondaryButton`/`ToggleButton`) on new theme tokens;
  applied to Anime header/setup/details and to the Watch transport row.
- **Latent (progress attribution).** `AppState.anime_pending_playing`;
  `flush_anime_progress()` in `main.rs`; identity promoted only at load time;
  cleared + flushed when leaving Watch and when starting YouTube; periodic save
  gated on `media_loaded && !transition_pending`.
- **P2 polish.** Card hover border (`border-strong`); shelf headings get an
  accent marker bar.
- **Tests.** All green (see `TEST_RESULTS.md`).

## Verified (automated)

`cargo fmt --all -- --check`, `cargo check --workspace` (no warnings),
`cargo test --workspace`, `cargo build -p mutsukitube-desktop`.

## NOT verified (needs a human at the window)

Visual/interaction behaviour: independent episode-list scrolling, button
hover/pressed/focus, slider feel mid-drag, at 1366×768 and 1920×1080. Marked
NOT_RUN; steps in `MANUAL_TEST_PLAN.md`.

## Known limitations (unchanged from 5.5)

No synopsis/genres/studio/rating/backdrop, flat episode list, no episode
thumbnails/durations, duration only after a play session, no Next/Previous in the
player, no scroll-position restore on back. Duplicate episode numbers (rare;
providers currently skip `number <= 0`) remain indistinguishable by number.

## Deferred until operator approval

Deploy the ASUS static plugin registry, provision the production Ed25519 key,
publish a signed release, LAN install test, and publish `source-gpl.tar.gz` +
`NOTICE` + `LICENSE`.

## Next actions

1. Human runs `MANUAL_TEST_PLAN.md`; record observations in `TEST_RESULTS.md`.
2. Only if a visual defect is observed, reopen the relevant checkpoint.
3. Operator-gated registry/key/publication work (above).

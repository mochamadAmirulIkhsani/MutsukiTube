# Phase 5.5 — Test Results

All commands run from `D:\Projects\mutsukitube-folder\mutsukitube` on Windows.

## Automated — last run this session (all PASS)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace` | Finished, exit 0, no warnings |
| `cargo test --workspace` | exit 0, all suites pass |
| `cargo build -p mutsukitube-desktop` | Finished, exit 0 |

`cargo test --workspace` suite breakdown:

| Suite | Passed |
|---|---|
| mutsukitube-anime (unit) | 39 |
| mutsukitube-anime (integration) | 33 |
| mutsukitube-embedded-player | 2 |
| mutsukitube-plugin-updater (unit) | 13 |
| mutsukitube-plugin-updater (integration, `go_manifest_compat`) | 4 |
| mutsukitube-storage | 10 |
| mutsukitube-youtube | 4 |
| mutsukitube-youtube `native_pagination` | 1 ignored (pre-existing) |

## New tests added in Phase 5.5

`crates/mutsukitube-storage/src/anime.rs` (in-memory SQLite, 8 tests):

- `progress_below_minimum_is_not_continue_watching`
- `in_progress_episode_appears_in_continue_watching`
- `completed_episode_leaves_continue_watching_but_stays_in_history`
- `progress_upsert_does_not_duplicate_rows`
- `progress_is_scoped_per_episode`
- `favorite_toggle_round_trips`
- `completion_threshold`
- `anime_key_is_stable`

`crates/mutsukitube-anime/tests/integration.rs` (1 test):

- `available_providers_require_existing_executable` — empty install dir yields
  zero available providers; a dir containing the real `ylnime` binary yields
  exactly `["ylnime"]`.

## What is NOT verified by automated tests

- The Slint UI was not rendered/observed. "Compiles and is wired" ≠ "pixels
  correct".
- No anime episode was streamed in this environment; embedded playback,
  resume-seek, and progress persistence were not observed end-to-end.
- No signed registry was deployed, so the install flow was exercised only via
  unit/integration tests and the existing signed-fixture tests, not a live
  download into the UI.
- Responsive behavior at 1366×768 / 1920×1080 / 125% / 150% scaling.

These require a human at the machine. See `MANUAL_TEST_PLAN.md`.

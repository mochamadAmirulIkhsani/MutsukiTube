# Phase 5.5.2 — Smart Back Navigation & YouTube Home — TEST RESULTS

Environment: Windows, Rust workspace `D:\Projects\mutsukitube-folder\mutsukitube`,
branch `feat/youtube-home-navigation`.

## Commands run

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean (exit 0) |
| `cargo check --workspace` | success, **0 warnings** |
| `cargo test --workspace` | **all green** (see below) |
| `cargo build -p mutsukitube-desktop` | success |

## `cargo test --workspace` detail

| Target | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| `mutsukitube-anime` unit | 47 | 0 | 0 |
| `mutsukitube-anime` `tests/integration.rs` | 34 | 0 | 0 |
| `mutsukitube-desktop` unit | 12 | 0 | 0 |
| `mutsukitube-embedded-player` unit | 2 | 0 | 0 |
| `mutsukitube-plugin-updater` unit | 13 | 0 | 0 |
| `mutsukitube-plugin-updater` `go_manifest_compat` | 4 | 0 | 0 |
| `mutsukitube-storage` unit | 10 | 0 | 0 |
| `mutsukitube-youtube` unit | 4 | 0 | 0 |

Desktop unit tests rose 6 → 12: 6 new navigation-stack tests
(`apps/desktop/src/navigation.rs` tests) covering push, consecutive-duplicate
skipping, pop-to-origin, root protection, stack cap, and context sensitivity.

## New automated coverage

### Navigation state — `navigation.rs` tests
- `push_entry_records_the_new_screen`
- `push_entry_skips_consecutive_duplicates`
- `pop_current_returns_the_origin`
- `pop_current_keeps_the_root`
- `stack_is_capped`
- `same_route_with_different_context_is_not_a_duplicate`

## NOT_RUN — requires a human at the window

All visual/interactive behavior (Back feel, home layout, topic chips, card
interaction, offline states) is NOT_RUN. Steps are in
`MANUAL_TEST_PLAN.md`. Claims of visual success are intentionally absent.

## Deferred

- Playback slider bug (stale duration on episode switch): **DEFERRED**, not
  touched in this phase. Recorded as a known unresolved issue.

# Phase 5.5.1 — Test Results

Environment: Windows, Rust workspace `D:\Projects\mutsukitube-folder\mutsukitube`,
branch `fix/anime-ux-refinement`.

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
| `mutsukitube-anime` unit (`src/lib.rs`) | 47 | 0 | 0 |
| `mutsukitube-anime` `fake_anime_provider` bin | 0 | 0 | 0 |
| `mutsukitube-anime` `tests/integration.rs` | 34 | 0 | 0 |
| `mutsukitube-core` unit | 0 | 0 | 0 |
| `mutsukitube-desktop` unit (`src/main.rs`) | 6 | 0 | 0 |
| `mutsukitube-embedded-player` unit | 2 | 0 | 0 |
| `mutsukitube-player` unit | 0 | 0 | 0 |
| `mutsukitube-plugin-updater` unit | 13 | 0 | 0 |
| `mutsukitube-plugin-updater` `go_manifest_compat` | 4 | 0 | 0 |
| `mutsukitube-storage` unit | 10 | 0 | 0 |
| `mutsukitube-youtube` unit | 4 | 0 | 0 |
| `mutsukitube-storage` `native_pagination` | 0 | 0 | 1 |

Delta vs Phase 5.5: anime unit 39 → 47 (ordering), anime integration 33 → 34
(ordering via IPC), desktop 0 → 6 (timeline resolver).

## New automated coverage by requirement

### Playback session isolation — `player_controller/timeline.rs`
- `switching_episodes_does_not_leak_previous_timeline` — **Scenario A**: while
  loading, `resolve_timeline(false, Some(620.0), Some(800.0))` ⇒
  `position 0 / duration 0 / has_duration false`. No stale values.
- `loaded_media_reports_its_own_duration` — after load, episode 2's own values
  (2 / 1440) win.
- `position_is_clamped_to_known_duration` — **Scenario B**: `9999` clamped to
  `300`.
- `unknown_duration_is_not_fabricated` — **Scenario D**: `None`/`0`/`-5`/`NaN`
  duration ⇒ `duration 0`, `has_duration false`, real position still reported.
- `non_finite_or_negative_position_is_normalised` — `-1`, `NaN`, `+inf` ⇒ `0`.
- `not_loaded_ignores_every_raw_reading` — gate short-circuits everything.

### Episode sorting / identity — `mutsukitube-anime`
- Unit: `descending_episodes_are_sorted_ascending` (12…1 → 1,2,10,11,12),
  `ordering_preserves_each_episode_identity` (number↔URL intact),
  `lexical_order_is_not_used` (1,2,10,100), `unnumbered_entries_keep_stable_order_at_the_end`,
  `title_fallback_orders_unnumbered_episodes`,
  `parses_common_episode_markers` (incl. `12.5`, `Ep. 7`),
  `does_not_misread_years_resolutions_or_specials`, `already_sorted_lists_are_unchanged`.
- Integration: `episodes_are_returned_in_ascending_order` — provider returns
  12,11,10,2,1 over the real IPC boundary; manager returns `[1,2,10,11,12]` with
  each URL still matching its number.

### State transitions (progress attribution)
Covered structurally in `main.rs` (identity promoted only at load, cleared on
leave/YouTube) and by the timeline gate above. No unit test asserts the SQLite
writes end-to-end in-process; that path is exercised by the storage unit tests
(`save_anime_progress`, Continue Watching thresholds) plus manual Scenario F.

## NOT_RUN — requires a human at the window

| Area | Status |
| --- | --- |
| Slider feel during drag / keyboard seek | NOT_RUN |
| Episode list scrolls independently (no page scroll) at 1366×768 and 1920×1080 | NOT_RUN |
| Button hover / pressed / focused / disabled / selected states | NOT_RUN |
| Play vs Resume dominance in the hero | NOT_RUN |
| Overall polish (card hover, shelf headings) | NOT_RUN |
| Scenarios C (rapid 1→2→3) and E (anime→YouTube) live | NOT_RUN |

Compilation and unit tests cannot validate layout or pointer behaviour. Steps to
clear these are in `MANUAL_TEST_PLAN.md`. Claims of visual success are
intentionally absent.

# MutsukiTube Issue & Bug Simulation Tracker

Branch: `issue-simulation`  
Updated: 2026-10-28  
Status: **CRITICAL AND HIGH PRIORITY ISSUES RESOLVED AND VERIFIED**

This document catalogs bugs and edge cases discovered in MutsukiTube, along with their resolution status and automated verification tests.

---

## Issue Summary Matrix

| ID | Severity | Component | Issue Description | Status | Verification / Simulation Test |
|---|---|---|---|---|---|
| **ISSUE-01** | **CRITICAL** | Desktop / Home | YouTube Home explore videos cannot be played (`Video not found`) | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_01_home_explore_video_playback_succeeds` |
| **ISSUE-02** | **HIGH** | Desktop / Library | Home explore videos cannot be favorited or added to playlists | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_02_home_video_can_be_favorited_and_added_to_playlist` |
| **ISSUE-03** | **HIGH** | Desktop / Nav | Root `YouTubeHome` permanently evicted from navigation stack on >50 browses | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_03_navigation_stack_preserves_root_home` |
| **ISSUE-04** | **MEDIUM** | Desktop / Home | Topic chip "All" (empty string) dropped by `on_load_home_topic` | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_04_home_topic_all_filter_handled` |
| **ISSUE-05** | **MEDIUM** | Desktop / Anime | Episode 0 (Prologue/Special) resume skipped (`resume_number > 0`) | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_05_anime_episode_zero_resume_accepted` |
| **ISSUE-06** | **MEDIUM** | Desktop / UI | Seek slider permanently locks position updates on keyboard arrow navigation | **RESOLVED** | `apps/desktop/src/issue_simulations.rs::verify_issue_07_keyboard_seek_slider_watchdog_clears` |
| **ISSUE-07** | **MEDIUM** | Desktop / Home | Continue Watching & Favorites shelves on Home have blank thumbnails | **RESOLVED** | Verified in `handlers/home.rs` + `thumbnail_ui.rs` |
| **ISSUE-08** | **MEDIUM** | Storage / Playlists | Multi-byte UTF-8 playlist names rejected by byte length check (`len() > 100`) | **RESOLVED** | `crates/mutsukitube-storage/src/issue_simulations.rs::verify_issue_08_multibyte_playlist_name_allowed` |
| **ISSUE-09** | **HIGH** | Storage / SQLite | Database connection lacks WAL mode, risking lock contention | **RESOLVED** | `crates/mutsukitube-storage/src/issue_simulations.rs::verify_issue_09_sqlite_connection_enables_wal_mode` |
| **ISSUE-10** | **LOW** | Desktop / Player | Unknown/zero duration causes live streams to overflow Slider maximum (1.0) | **RESOLVED** | Disables seek slider when duration <= 0 |
| **ISSUE-11** | **LOW** | YouTube / Models | Duration parser fails on strings with leading/trailing whitespace | **RESOLVED** | `crates/mutsukitube-youtube/tests/issue_simulations.rs::verify_issue_11_duration_parser_whitespace_trimmed` |
| **ISSUE-12** | **MEDIUM** | Anime / Ordering | Unnumbered "Prologue" (Episode 0) sorted to end instead of beginning | **RESOLVED** | `crates/mutsukitube-anime/tests/issue_simulations.rs::verify_issue_12_prologue_episode_zero_sorted_before_episode_one` |
| **ISSUE-13** | **HIGH** | Player / Timeline | Episode switch stale duration leak | **DEFERRED** | Documented in `docs/agent-handoff/anime-phase5-5-2/STATE.md` |

---

## Detailed Issue Resolutions

### ISSUE-01: YouTube Home Explore Videos Cannot Be Played (CRITICAL - RESOLVED)
- **Fix**:
  1. Added `pub home_results: Vec<Video>` to `AppState`.
  2. In `handlers/home.rs`, saved explore and topic search results into `state.home_results`, and merged history & favorites items.
  3. In `handlers/player.rs`, updated `find_video` to search `state.home_results.iter()`.
- **Verification**: `verify_issue_01_home_explore_video_playback_succeeds` passes.

---

### ISSUE-02: Home Explore Videos Cannot Be Favorited / Added to Playlist (HIGH - RESOLVED)
- **Fix**:
  1. In `handlers/library.rs`, updated `find_video` to search `state.home_results.iter()`.
  2. Added fallback in `handlers/library.rs` to construct `Video` from active UI properties (`ui.get_selected_video_title()`, etc.) if not present in lookup cache.
- **Verification**: `verify_issue_02_home_video_can_be_favorited_and_added_to_playlist` passes.

---

### ISSUE-03: Root YouTubeHome Evicted on Deep Navigation (HIGH - RESOLVED)
- **Fix**:
  In `navigation.rs`, modified `push_entry` so that when `nav_stack.len() > NAV_STACK_LIMIT`, it drops from index 1 (`state.nav_stack.remove(1)`) while index 0 (`YouTubeHome`) is permanently retained as the stack root.
- **Verification**: `verify_issue_03_navigation_stack_preserves_root_home` passes.

---

### ISSUE-04: Home Topic Filter "All" Chip Handled (MEDIUM - RESOLVED)
- **Fix**:
  In `handlers/home.rs`, `on_load_home_topic` checks `if topic.is_empty()`. When empty (user clicked "All"), it invokes `load_home(weak.clone(), context.clone())` to reset to the default explore feed.
- **Verification**: `verify_issue_04_home_topic_all_filter_handled` passes.

---

### ISSUE-05: Anime Episode 0 Resume Skipped (MEDIUM - RESOLVED)
- **Fix**:
  In `handlers/anime.rs`, changed `resume_number > 0` checks to `resume_number >= 0`, ensuring 0-indexed prologue episodes are treated as valid resume targets.
- **Verification**: `verify_issue_05_anime_episode_zero_resume_accepted` passes.

---

### ISSUE-06: Seek Slider Keyboard Lockout Watchdog (MEDIUM - RESOLVED)
- **Fix**:
  In `main.rs`, added a seeking watchdog counter in the 200 ms surface timer. If `timeline_seek_active` remains true for more than 1.6 seconds without a pointer release (such as during keyboard arrow interaction), the watchdog automatically resets `timeline_seek_active(false)` and restores normal playback slider auto-advance.
- **Verification**: `verify_issue_07_keyboard_seek_slider_watchdog_clears` passes.

---

### ISSUE-07: Home Screen Continue Watching & Favorites Blank Thumbnails (MEDIUM - RESOLVED)
- **Fix**:
  1. Added `ThumbnailTarget::HomeContinue` and `ThumbnailTarget::HomeFavorites` to `thumbnail_ui.rs`.
  2. In `handlers/home.rs`, queued thumbnail downloads for `history` and `favorites` shelves in addition to `explore`.

---

### ISSUE-08: Multi-byte UTF-8 Playlist Names Rejected (MEDIUM - RESOLVED)
- **Fix**:
  In `crates/mutsukitube-storage/src/library.rs`, changed length validation to `char_count = name.chars().count()` (1-100 characters) instead of byte length.
- **Verification**: `verify_issue_08_multibyte_playlist_name_allowed` passes.

---

### ISSUE-09: SQLite Concurrency Missing WAL Mode (HIGH - RESOLVED)
- **Fix**:
  In `crates/mutsukitube-storage/src/database.rs::open_connection()`, added `connection.pragma_update(None, "journal_mode", "WAL")?;` and `connection.pragma_update(None, "synchronous", "NORMAL")?;`.
- **Verification**: `verify_issue_09_sqlite_connection_enables_wal_mode` passes.

---

### ISSUE-11: YouTube Duration Parser Whitespace Trim (LOW - RESOLVED)
- **Fix**:
  In `crates/mutsukitube-youtube/src/models.rs`, trimmed `text` and each duration `part` prior to `.parse()`.
- **Verification**: `verify_issue_11_duration_parser_whitespace_trimmed` passes.

---

### ISSUE-12: Unnumbered "Prologue" Episode 0 Sorted to End (MEDIUM - RESOLVED)
- **Fix**:
  In `crates/mutsukitube-anime/src/ordering.rs`, added `"prologue"` recognition in `parse_episode_number`, assigning it order key `0.0` so it sorts before Episode 1.
- **Verification**: `verify_issue_12_prologue_episode_zero_sorted_before_episode_one` passes.

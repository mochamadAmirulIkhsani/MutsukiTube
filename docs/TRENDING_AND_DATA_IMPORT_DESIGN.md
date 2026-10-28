# MutsukiTube Design Specification: Trending Page, Extractor Resilience & Data Importer

Updated: 2026-10-28  
Status: **IMPLEMENTED & VERIFIED**  
Branch: `trending-and-data-import`  
Related Documents: [`docs/ISSUES.md`](ISSUES.md)

---

## 1. Executive Summary

This specification outlines the implementation for MutsukiTube, covering three interconnected features:
1. **Dedicated YouTube Trending Page (Option A)**: A full-featured Trending destination in the primary sidebar supporting regional charts (Indonesia `ID` / Global `US`) and categories (`Now`, `Music`, `Gaming`, `Movies`).
2. **Extractor Bot-Verification Mitigation**: Hardening `EmbeddedMpvPlayer` and `ExternalMpvPlayer` with client spoofing (`ytdl_hook-ytdl_args` / `ytdl-raw-options`) to prevent YouTube's bot-detection challenges (`Sign in to confirm you're not a bot`).
3. **Language-Agnostic Google Takeout Importer & Settings UI**: A robust parser and Settings UI for Google Takeout CSV exports supporting localized headers (e.g. Indonesian `langganan.csv` with `ID Channel,URL Channel,Judul Channel` and English `subscriptions.csv`).

---

## 2. Feature 1: Dedicated YouTube Trending Page (Option A)

### 2.1 Navigation & Placement
- Added as a top-level `NavItem` in [`apps/desktop/ui/components/sidebar.slint`](../apps/desktop/ui/components/sidebar.slint) directly between **Home** and **Search**.
- Uses the flame icon asset: [`apps/desktop/ui/assets/trending.svg`](../apps/desktop/ui/assets/trending.svg).
- Active state highlighted when `current-page == "trending"`.

### 2.2 Navigation Model Integration (`apps/desktop/src/navigation.rs`)
```rust
pub enum NavRoute {
    YouTubeHome,
    YouTubeTrending { category: String },
    YouTubeSearch { query: String },
    // ...
}
```
- Page identifier: `"trending"`.
- Context restoration preserves selected category and cached video list so navigating `Trending -> WatchPage -> Back` returns instantly without re-fetching.

### 2.3 InnerTube & yt-dlp Resilient Backend Integration (`crates/mutsukitube-youtube`)
- **Root Cause of Upstream Deprecation**: As documented by FreeTube and YouTube.js issue #1114, YouTube recently deprecated the standalone `FEtrending` InnerTube browse endpoint (HTTP 400 `INVALID_ARGUMENT`) and redirects `/feed/trending` to the home page (`/`), causing `yt-dlp` to fail with:
  `ERROR: [youtube:tab] trending: The channel/playlist does not exist and the URL redirected to youtube.com home page`.
- **Search Query Mapping Architecture**:
  Both Native InnerTube and `yt-dlp` fallbacks query YouTube's active search endpoints using localized trending queries and regional geolocation (`gl: "ID"` or `"US"`):
  | Category | Indonesia (`ID`) | Global (`US`) |
  |---|---|---|
  | **Now (All)** | `Trending Indonesia` | `Trending` |
  | **Music** | `Trending Musik Indonesia` | `Trending Music` |
  | **Gaming** | `Trending Gaming Indonesia` | `Trending Gaming` |
  | **Movies** | `Trending Movie Trailers Indonesia` | `Trending Movie Trailers` |
- **Native InnerTube**:
  - `POST https://www.youtube.com/youtubei/v1/search?prettyPrint=false` with `params: "CAISAhAB"` (video filter) and regional client context `"gl": country`.
- **yt-dlp Fallback**:
  - `ytsearch24:{query}` with `--dump-json --skip-download --no-warnings`.
- **Renderer Parsing (`models.rs`)**:
  Extended `collect_videos` to match:
  - `videoRenderer` (Standard feed)
  - `gridVideoRenderer` (Trending grid shelves)
  - `compactVideoRenderer` (Compact mobile / list feeds)

### 2.4 UI Layout (`apps/desktop/ui/pages/trending-page.slint`)
- **Top Bar**:
  - Heading: "Trending" with regional subtitle ("Popular right now in Indonesia" or "Popular right now worldwide (US)").
  - Country Toggle Pill: `🇮🇩 ID (Indonesia)` / `🌐 Global (US)`.
  - Refresh button.
- **Category Pills**: Segmented pill row (`Now`, `Music`, `Gaming`, `Movies`).
- **Video Grid**: Responsive 4-column layout utilizing `GridVideoCard`.
- **Status & Loading**: Spinner indicator and result count status text.

### 2.5 State & Thumbnail Flow (`handlers/trending.rs`)
- `AppState.trending_results: Vec<Video>` stores the active feed for instant playback lookup.
- Asynchronous thumbnail downloads dispatched through `queue_thumbnails(..., ThumbnailTarget::Trending)`.

---

## 3. Feature 2: Extractor Bot-Verification Mitigation

### 3.1 Problem Statement
When `EmbeddedMpvPlayer` loads a raw YouTube URL `https://www.youtube.com/watch?v=...`, `mpv`'s `ytdl_hook` invokes `yt-dlp` using YouTube's desktop `WEB` client. YouTube triggers bot verification:
```
ERROR: [youtube] SYXTo-iOhWE: Sign in to confirm you're not a bot.
mpv: Failed to recognize file format
```

### 3.2 Mitigation in `EmbeddedMpvPlayer` & `ExternalMpvPlayer`
1. In [`crates/mutsukitube-embedded-player/src/lib.rs`](../crates/mutsukitube-embedded-player/src/lib.rs), dynamically configure `ytdl_hook` script options on `Mpv`:
```rust
mpv.command(&[
    "change-list",
    "script-opts",
    "append",
    "ytdl_hook-ytdl_args=--extractor-args=youtube:player_client=android,ios,web",
])?;
```
2. In [`crates/mutsukitube-player/src/lib.rs`](../crates/mutsukitube-player/src/lib.rs) and [`crates/mutsukitube-core/src/lib.rs`](../crates/mutsukitube-core/src/lib.rs):
```rust
Command::new("mpv")
    .arg("--ytdl-raw-options=extractor-args=youtube:player_client=android,ios,web")
```
- Switches primary stream extraction to YouTube's Android and iOS client endpoints.
- Bypasses desktop Botguard attestation requirements without requiring user login or cookies.

---

## 4. Feature 3: Language-Agnostic Google Takeout Importer & Settings UI

### 4.1 Problem Statement
Google Takeout localizes both file names and CSV column headers based on account language:
- **English**: `subscriptions.csv` with headers `Channel Id,Channel Url,Channel Title`
- **Indonesian**: `langganan.csv` with headers `ID Channel,URL Channel,Judul Channel`

A naive header string match fails for non-English users, and users need an accessible UI in Settings to select or paste their CSV path and trigger imports.

### 4.2 Positional & Pattern-Matching Solution
In [`crates/mutsukitube-storage/src/importer.rs`](../crates/mutsukitube-storage/src/importer.rs):
1. **Accept Any CSV Filename**: Works with `langganan.csv`, `subscriptions.csv`, or custom filenames.
2. **UTF-8 BOM Trimming**: Strips `\u{feff}` BOM present in Windows Takeout exports.
3. **Column Position Mapping**:
   - Column 0: Channel ID
   - Column 1: Channel URL
   - Column 2: Channel Title
4. **Pattern Validation**:
   - Verify Column 0 starts with `"UC"` and has a length of 24 characters.
   - Header rows (`ID Channel`, `Channel Id`, etc.) are naturally ignored because they fail the channel ID pattern check.
5. **Atomic Batch Insertion**:
   - Writes imported channels into the `subscriptions` table inside a single `rusqlite::Transaction`.
6. **Count & Clear API**:
   - `get_subscriptions_count() -> rusqlite::Result<usize>`
   - `clear_subscriptions() -> rusqlite::Result<usize>`

### 4.3 Settings Page UI Integration
In [`apps/desktop/ui/pages/settings-page.slint`](../apps/desktop/ui/pages/settings-page.slint) and [`apps/desktop/src/handlers/settings.rs`](../apps/desktop/src/handlers/settings.rs):
- **Import Card**: Grouped Cupertino card displaying:
  - Current status: `X channels saved` / `No subscriptions imported`.
  - Explanatory subtitle for Google Takeout export instructions.
  - Path `LineEdit` for manual path entry / pasting.
  - `Browse...` button triggering Windows native `System.Windows.Forms.OpenFileDialog` via PowerShell in background thread.
  - Primary `Import CSV` action button with live loading state and success/failure alerts.
  - `Clear Subscriptions` button when saved channels exist.

---

## 5. Implementation Roadmap & Checklist

| Phase | Task | Files Involved | Status |
|---|---|---|---|
| **Phase 1** | Implement `ytdl_hook` extractor fallback | `crates/mutsukitube-embedded-player/src/lib.rs`, `mutsukitube-player` | **RESOLVED** |
| **Phase 2** | Add resilient `trending` search API & renderer parsing | `crates/mutsukitube-youtube/src/native.rs`, `models.rs`, `ytdlp.rs`, `hybrid.rs` | **RESOLVED** |
| **Phase 3** | Create `trending.svg` icon asset | `apps/desktop/ui/assets/trending.svg` | **RESOLVED** |
| **Phase 4** | Build `trending-page.slint` UI & Sidebar NavItem | `apps/desktop/ui/pages/trending-page.slint`, `sidebar.slint`, `app.slint` | **RESOLVED** |
| **Phase 5** | Add `NavRoute::YouTubeTrending` & handler logic | `apps/desktop/src/navigation.rs`, `handlers/trending.rs`, `main.rs`, `state.rs` | **RESOLVED** |
| **Phase 6** | Implement `langganan.csv` / `subscriptions.csv` importer & Settings UI | `crates/mutsukitube-storage/src/importer.rs`, `settings-page.slint`, `handlers/settings.rs` | **RESOLVED** |
| **Phase 7** | Verification & Automated Tests | `apps/desktop/src/issue_simulations.rs`, `crates/*/tests` | **VERIFIED** |

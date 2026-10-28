# Phase 5.5 — Anime Experience — STATE

Authoritative resume state. Keep this file current.

## Repository

- Rust repo: `D:\Projects\mutsukitube-folder\mutsukitube`
- Branch: `feat/anime-ux-phase5-5` (local only — **not pushed**)
- Base: `ddddc677f5c421585ee5a5c3f172a3f44dc7e902`
- Code + docs commit: `6b82fd3b3ed234d52f85fe38310ba3530deec353`
- Follow-up fix commit: `4d443b9` (view switching via conditionals; resume from search)

## Objective

Turn the Anime section into a Jellyfin-inspired, fully connected native media
library: poster grids, a hero details view, an episode browser, Continue
Watching/history/favorites from durable local state, search, and plugin
onboarding — preserving embedded libmpv, Go providers, the signed updater, and
all YouTube behavior. No Electron/WebView/HTML. No fabricated metadata.

## Done this session

- Design system: `ui/theme.slint` + `AnimePosterCard`, `EpisodeRow`,
  `PosterShelf` components.
- `ui/pages/anime-page.slint` rebuilt: header (Back/title/search), status bar,
  and `setup` / `details` / `home` states.
- `ui/app.slint`: anime view-model properties and callbacks wired.
- `src/state.rs`: `AppState` extended (view, cards map, resume map,
  selected episode, now-playing, pending resume, progress-save clock).
- `src/handlers/anime.rs`: home refresh, search→cards, details + episodes +
  progress status, favorite toggle, play/resume, install flow (preserved).
- `src/main.rs`: persist progress ~every 5s while playing; apply resume seek.
- `src/native_window/session.rs`: `pending_anime_resume`.
- `crates/mutsukitube-storage/src/anime.rs`: `anime_history`, `anime_progress`,
  `anime_favorites` tables + helpers + 8 tests.
- `crates/mutsukitube-anime`: `AnimeManager::available_provider_ids` + test.
- Docs: this folder (`UX_AUDIT`, `DESIGN_SYSTEM`, `CHECKPOINTS`, `TEST_RESULTS`,
  `MANUAL_TEST_PLAN`).

## Verified

- `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo test --workspace`, `cargo build -p mutsukitube-desktop` — all pass.
- Storage round trips and provider-availability logic have automated tests.

## Not verified (needs a human)

- Any visual/rendering claim (window not observed).
- Real anime playback, resume-seek, and progress persistence end-to-end.
- Live signed-registry install into the UI.
- Responsive layout at target resolutions/scaling.

## Blocked / out of scope (unchanged from Phase 5)

- ASUS static registry deployment not approved/performed.
- Production Ed25519 public key not provisioned.
- No ASUS/Docker/firewall/router changes.

## Next move

1. Run `MANUAL_TEST_PLAN.md` and record real observations in `TEST_RESULTS.md`.
2. Fix anything the manual run surfaces (layout regressions, image loading).
3. Optionally implement Next/Previous Episode on the Watch page (only if wired
   to real state — no decorative buttons).
4. When approved: deploy the registry, publish a signed release, LAN install.

## Constraints (do not violate)

- Never push. Local commits only. No destructive git.
- Go repo stays private; no visibility changes.
- Preserve YouTube, embedded libmpv, AnimeManager, provider fallback, existing UI.
- Never commit/log private keys, tokens, credentials, or full signed media URLs.
- Do not claim playback/deployment success without real observation.

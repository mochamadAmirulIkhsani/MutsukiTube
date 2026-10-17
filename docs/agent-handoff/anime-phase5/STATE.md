# MutsukiTube Phase 5 — Signed Plugin Distribution & Homelab

## Current State
- Current checkpoint: CP-03 (deployment readiness) / CP-06/07/08 local work complete
- Checkpoint status: CP-06 COMPLETED(local); CP-07 COMPLETED(local);
  CP-08 COMPLETED(local) / BLOCKED(LAN); CP-03 BLOCKED(approval)
- Last completed checkpoint (local): CP-08 local verification
- Last update: 2026-10-18 (session 2)
- Active Git branch: feat/anime-plugin-distribution
- Latest local commit: <filled after commit>
- Working tree status: modified (continuation changes uncommitted)
- Resume procedure: read this file, CHECKPOINTS.md, TEST_RESULTS.md, then
  HOMELAB_DEPLOYMENT_PLAN.md and LICENSING.md

## Project Locations
- Rust repository: D:\Projects\mutsukitube-folder\mutsukitube
- Go repository: D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins
- Homelab: ikhsan@192.168.18.11 (X555BP, Debian 13) — SSH read-only verified

## Implementation Summary
- Completed (local):
  - Rust `mutsukitube-plugin-updater` crate
  - Ed25519 manifest verification, schema/path/URL/platform validation
  - SHA-256 artifact verification, size limits
  - Atomic versioned install + activation
  - Version state, previous-known-good retention
  - Automatic rollback on failed startup/IPC handshake
  - Known-bad release guard, interrupted-install safety
  - Offline startup using existing verified plugins
  - Desktop Install Plugins workflow with status/progress strings
  - Production trust anchor: env override > compile-time key > dev key (debug only)
  - Cross-language signing verified (Go signs, Rust verifies)
  - Homelab read-only inspection + deployment plan + licensing plan
- Blocked:
  - ASUS registry deployment (needs operator approval)
  - LAN retrieval test (needs deployment)
  - Public HTTPS endpoint (separate approval)

## Changed Files (this continuation)
- crates/mutsukitube-plugin-updater/src/lib.rs (rollback, state, trust anchor)
- crates/mutsukitube-plugin-updater/tests/go_manifest_compat.rs (new)
- crates/mutsukitube-plugin-updater/tests/fixtures/manifest.json (new, public)
- crates/mutsukitube-plugin-updater/tests/fixtures/manifest.sig (new, public)
- crates/mutsukitube-anime/Cargo.toml (dev-dependency on updater)
- crates/mutsukitube-anime/tests/integration.rs (updater->manager chain test)
- apps/desktop/src/handlers/anime.rs (UX states, offline message, bad-release guard)
- apps/desktop/src/main.rs (startup validation + auto-rollback)
- docs/agent-handoff/anime-phase5/STATE.md
- docs/agent-handoff/anime-phase5/CHECKPOINTS.md
- docs/agent-handoff/anime-phase5/TEST_RESULTS.md
- docs/agent-handoff/anime-phase5/HOMELAB_DEPLOYMENT_PLAN.md (new)
- docs/agent-handoff/anime-phase5/LICENSING.md (new)

## Architecture Decisions
- Homelab stays a static artifact server only
- Plugin execution stays on the desktop
- Manifest Ed25519 signature is the load-bearing trust boundary
- Production requires a trusted key; dev key only in debug builds
- Production registry URLs must be HTTPS (loopback HTTP allowed for dev)
- Local dev override `MUTSUKITUBE_ANIME_PLUGIN_DIR` still wins
- Rollback triggers only on startup/IPC failure or tamper, not on site outage

## Verification Results (session 2)
- cargo fmt --all -- --check: PASS
- cargo check --workspace: PASS
- cargo test --workspace: PASS
  - anime unit 39; anime integration 32 (incl. real Go handshake)
  - updater unit 13; updater integration 4 (Go-signed manifest compat)
  - embedded-player 2
- cargo build -p mutsukitube-desktop: PASS
- go test ./...: PASS
- go vet ./...: PASS
- Go sign -> Rust verify cross-check: PASS

## Homelab Findings
- No system nginx/caddy/apache; Docker socket not accessible to ikhsan
- No passwordless sudo; python3 and busybox available
- Candidate registry dir: /srv/storage/nas/mutsukitube-plugins (ikhsan-owned)
- Recommended static server: busybox httpd on port 8088 via systemd --user
- Occupied ports: 22, 53, 139, 445, 8080, 8096, 8123, 9090, 18554, 18555
- No server mutation performed

## Security Decisions
- Signature verified before any metadata is trusted
- SHA-256 verified before install
- Unsafe paths/URLs/platforms and oversized downloads rejected
- No private keys or credentials committed or logged
- `install_artifact` refuses to lower the active version

## Known Issues / Still Open
- No explicit cancel button or numeric progress in the install UI
- No background update scheduler
- Full GPL source-archive automation not implemented (plan documented)
- Manual anime playback after first-install not observed in this environment
- LAN deployment not performed

## Exact Next Actions
1. Operator approves ASUS registry deployment (see HOMELAB_DEPLOYMENT_PLAN.md).
2. Provide production Ed25519 public key; build desktop with it embedded.
3. Deploy registry, publish a signed release, run LAN retrieval + install test.
4. Manually verify anime playback after an updater-installed plugin.
5. If approved, publish source-gpl.tar.gz + NOTICE + LICENSE per LICENSING.md.

## Important Restrictions
- Preserve YouTube, libmpv, AnimeManager fallback, existing UI
- Do not deploy to homelab without approval
- Do not push to GitHub
- Do not log secrets or signed media URLs

# MutsukiTube Phase 5 — Checkpoint History

## CP-00 — Baseline Audit & Homelab Inspection
- Status: COMPLETED
- Date/time: 2026-10-18
- Implemented functionality: Initial phase setup and read-only homelab inspection
- Files created: STATE.md; CHECKPOINTS.md; TEST_RESULTS.md
- Files modified: none
- Tests executed: none
- Actual test results: not run
- Known problems: None
- Git commit hash: not created
- Next checkpoint: CP-01

## CP-01 — Release Manifest & Security Contract
- Status: COMPLETED
- Implemented: Reused Go manifest schema; Rust verifier checks schema, signature, paths, URLs, platforms
- Files: crates/mutsukitube-plugin-updater/src/lib.rs
- Tests: cargo test -p mutsukitube-plugin-updater
- Result: PASS
- Next checkpoint: CP-02

## CP-02 — Go Build, Packaging & Signing
- Status: COMPLETED
- Implemented: Existing Go build/sign/verify tooling retained and tested
- Files: Go repository
- Tests: go test ./...; go vet ./...
- Result: PASS
- Next checkpoint: CP-03

## CP-03 — Homelab Static Registry Preparation
- Status: BLOCKED
- Implemented: Read-only SSH inspection and deployment plan documentation
- Files: docs/agent-handoff/anime-phase5/*
- Tests: SSH inspection
- Result: PASS (read-only)
- Known problems: Server-side deployment not approved or performed
- Next checkpoint: CP-04

## CP-04 — Rust Signed Manifest Verification
- Status: COMPLETED
- Implemented: Ed25519 verification, manifest parsing, platform selection, anti-rollback helper
- Files: crates/mutsukitube-plugin-updater/src/lib.rs
- Tests: cargo test -p mutsukitube-plugin-updater
- Result: PASS
- Next checkpoint: CP-05

## CP-05 — Secure Plugin Downloader & Atomic Installer
- Status: COMPLETED
- Implemented: HTTPS download, size limits, SHA-256 verification, atomic versioned install, activation
- Files: crates/mutsukitube-plugin-updater/src/lib.rs
- Tests: cargo test -p mutsukitube-plugin-updater
- Result: PASS
- Next checkpoint: CP-06

## CP-06 — Version Management, Rollback & Offline Support
- Status: PARTIALLY_IMPLEMENTED
- Implemented: Version state, activation, anti-rollback helper
- Files: crates/mutsukitube-plugin-updater/src/lib.rs
- Tests: updater unit tests
- Result: PASS
- Known problems: Full rollback automation and offline UI policy not implemented
- Next checkpoint: CP-07

## CP-07 — Slint Plugin Installation & Update UX
- Status: PARTIALLY_IMPLEMENTED
- Implemented: Install Plugins button and updater workflow handler
- Files: apps/desktop/ui/pages/anime-page.slint; apps/desktop/src/handlers/anime.rs
- Tests: cargo build -p mutsukitube-desktop
- Result: PASS
- Known problems: No progress UI or automatic update scheduler
- Next checkpoint: CP-08

## CP-08 — End-to-End Local and LAN Verification
- Status: PARTIALLY_IMPLEMENTED
- Implemented: Local updater tests; LAN deployment not performed
- Files: none
- Tests: cargo test --workspace
- Result: PASS
- Known problems: LAN registry test blocked on server deployment approval
- Next checkpoint: CP-09

## CP-09 — Security Hardening & Regression Testing
- Status: COMPLETED
- Implemented: Path/URL validation, signature/checksum verification, size limits, regression tests
- Files: crates/mutsukitube-plugin-updater/src/lib.rs
- Tests: cargo test --workspace; go test ./...
- Result: PASS
- Next checkpoint: CP-10

## CP-10 — Final Documentation & Handoff
- Status: COMPLETED
- Implemented: Phase 5 checkpoint documents
- Files: docs/agent-handoff/anime-phase5/*
- Tests: not applicable
- Result: DOCUMENTED
- Next checkpoint: none

---

# Continuation — 2026-10-18 (session 2): finish CP-03/06/07/08

No earlier record was rewritten. The entries below record new work only.

## CP-06 (update) — Version Management, Rollback & Offline Support
- Status: COMPLETED (local)
- Date/time: 2026-10-18
- Implemented functionality:
  - `mark_version_failed` demotes a bad active version and records it
  - `rollback_to_previous` restores the previous known-good version
  - `is_version_failed` blocks reinstalling a known-bad release
  - `activate_version` preserves the previous version for rollback
  - Interrupted installs leave the active version untouched (temp file staging)
  - Desktop startup validates the active version with an IPC `info` handshake and
    auto-rolls-back on failure; falls back to the installed dir when no state exists
  - Offline startup keeps existing verified plugins usable (no network required)
- Files modified: crates/mutsukitube-plugin-updater/src/lib.rs; apps/desktop/src/main.rs;
  apps/desktop/src/handlers/anime.rs
- Tests executed: cargo test -p mutsukitube-plugin-updater (13 unit)
- Actual results: PASS
- Explicitly NOT rolled back on: external anime site unavailability (only IPC/startup
  failure or tamper/signature failure triggers rollback)
- Git commit hash: recorded in STATE.md
- Next checkpoint: CP-07

## CP-07 (update) — Slint Plugin Installation & Update UX
- Status: COMPLETED (local)
- Implemented functionality:
  - Install Plugins action reports: checking registry, downloading per provider,
    verifying, installing, ready
  - Failure classification: "Registry offline or unreachable" for network errors,
    signature/checksum/install errors otherwise
  - Known-bad release guard prevents automatic reinstall
  - Retry is available by clicking Install Plugins again
  - Existing YouTube and Anime UI preserved; no provider/mirror selector added
- Files modified: apps/desktop/src/handlers/anime.rs; apps/desktop/ui/pages/anime-page.slint;
  apps/desktop/ui/app.slint
- Tests executed: cargo build -p mutsukitube-desktop; cargo check --workspace
- Actual results: PASS
- Known limitations: no progress bar/percentage and no explicit cancel button yet;
  no background update scheduler
- Next checkpoint: CP-08

## CP-08 (update) — End-to-End Local and LAN Verification
- Status: COMPLETED (local) / BLOCKED (LAN)
- Implemented functionality / evidence:
  - Local HTTP install flow test (manifest fetch -> Ed25519 verify -> artifact
    download -> SHA-256 verify -> atomic install -> activate): PASS
  - Go-signed manifest verified by the Rust verifier (cross-language): PASS
  - Tampered Go manifest rejected; wrong public key rejected: PASS
  - Updater install -> AnimeManager discovery -> real Go IPC `info` handshake: PASS
  - Rollback / known-bad / interrupted-install unit tests: PASS
- Files: crates/mutsukitube-plugin-updater/tests/go_manifest_compat.rs;
  crates/mutsukitube-plugin-updater/tests/fixtures/*; crates/mutsukitube-anime/tests/integration.rs
- Tests executed: cargo test --workspace
- Actual results: PASS (32 anime integration incl. real Go handshake; 13 updater unit;
  4 updater integration)
- LAN test: NOT_RUN — ASUS registry not deployed (approval required)
- Next checkpoint: CP-03 (deployment readiness)

## CP-03 (update) — Homelab Static Registry Preparation
- Status: BLOCKED (awaiting operator approval)
- Date/time: 2026-10-18
- Additional read-only findings:
  - No system nginx/caddy/apache installed
  - Docker socket not accessible to `ikhsan`; `sudo -n` requires a password
  - `python3` and `busybox` available; no extra package needed for a static server
  - `/srv/storage/nas` is ikhsan-owned with ~865 GB free
  - Candidate registry dir: `/srv/storage/nas/mutsukitube-plugins`
  - Recommended unprivileged static server: `busybox httpd` on port 8088 via a
    systemd user unit
- Files created: docs/agent-handoff/anime-phase5/HOMELAB_DEPLOYMENT_PLAN.md
- Files created: docs/agent-handoff/anime-phase5/LICENSING.md
- Tests executed: read-only SSH inspection only
- Actual results: plan prepared; no server mutation performed
- Required approval: create registry dir, run static server, publish artifacts,
  provide production public key
- Next checkpoint: operator approval, then LAN verification


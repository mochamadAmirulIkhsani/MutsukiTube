# MutsukiTube Phase 5 — Test Results

## 2026-10-18 — Homelab read-only inspection
- Working directory: Windows PowerShell
- Command: ssh ikhsan@192.168.18.11 ...
- Status: PASS
- Relevant output: Debian 13; 10 GiB RAM; 90 GiB root and 865 GiB /srv/storage available; docker command present but socket permission denied
- Known environmental limitations: Docker inspection unavailable to current SSH user

## 2026-10-18 (session 2) — Automated verification

### Rust workspace
- Command: `cargo fmt --all -- --check`
- Status: PASS

- Command: `cargo check --workspace`
- Status: PASS

- Command: `cargo test --workspace`
- Status: PASS
- Key results:
  - mutsukitube-anime unit: 39 passed
  - mutsukitube-anime integration: 32 passed (includes real Go binary discovery
    and Rust-Go IPC `info` handshake, plus updater->manager chain)
  - mutsukitube-plugin-updater unit: 13 passed (rollback, anti-rollback,
    known-bad guard, interrupted install, atomic install, local HTTP flow)
  - mutsukitube-plugin-updater integration: 4 passed (Go-signed manifest
    verifies in Rust; tampered manifest rejected; wrong key rejected)
  - mutsukitube-embedded-player: 2 passed
- Limitations: the real-plugin tests skip when the Go binary is absent; in this
  run the binary was present and the tests executed.

### Desktop build
- Command: `cargo build -p mutsukitube-desktop`
- Status: PASS

### Go repository
- Command: `go test ./...`
- Status: PASS

- Command: `go vet ./...`
- Status: PASS

### Cross-language signing
- Command: `go run ./cmd/mtb-manifest sign -manifest tests/fixtures/manifest.json`
  then Rust `verify_manifest(...)` with the development public key.
- Status: PASS — Go-produced Ed25519 signature and manifest schema accepted by Rust.

### Security tests (actual results)
- Invalid signature: PASS (rejected)
- Tampered manifest: PASS (rejected)
- Wrong public key: PASS (rejected)
- Tampered binary / checksum mismatch: PASS (rejected)
- Unsafe path / path traversal: PASS (rejected)
- Unsafe URL scheme: PASS (rejected)
- Unsupported platform selection: PASS (filtered)
- Anti-rollback (older version): PASS (rejected)
- Known-bad release reinstall: PASS (blocked)
- Interrupted install: PASS (old version stays active)

### NOT RUN / BLOCKED
- LAN retrieval from ASUS registry: BLOCKED (server deployment not approved)
- Real manual anime playback after first-install: NOT_RUN in this environment
- Public HTTPS registry: NOT_RUN (not designed/approved)

### Local first-install chain coverage
1. Fresh install without plugins — covered by design (env override + installed dir)
2. User clicks Install Plugins — wired (manual UI action)
3. Signed manifest retrieved — `local_http_install_flow` PASS
4. Ed25519 signature verified — `local_http_install_flow` + `go_manifest_compat` PASS
5. Compatible binary downloaded — `local_http_install_flow` PASS
6. SHA-256 verified — `local_http_install_flow` + `verify_artifact_bytes` PASS
7. Plugin installed atomically — `installs_atomically` PASS
8. AnimeManager discovers installed executable — `installed_version_directory...` PASS
9. Rust-Go IPC info handshake — `updater_install_then_manager_handshake` PASS
10. Anime search works — `real_go_provider_smoke` PASS (network-dependent)


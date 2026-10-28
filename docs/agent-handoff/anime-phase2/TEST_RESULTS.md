# MutsukiTube Phase 2 — Test Results

## 2026-10-18 02:01:51 — Repository inspection
- Working directory: D:\Projects\mutsukitube-folder
- Command: git status --short --branch (Rust and Go)
- Status: PASS
- Relevant output: Rust on feat/anime-plugin-host, clean; Go on main, clean
- Known environmental limitations: None recorded yet

## 2026-10-18 02:05:00 — Go baseline verification
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins
- Command: go test ./...
- Status: PASS
- Relevant output: all packages ok; cmd providers no test files
- Known environmental limitations: None

## 2026-10-18 02:05:30 — Go vet
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins
- Command: go vet ./...
- Status: PASS
- Relevant output: no vet findings
- Known environmental limitations: None

## 2026-10-18 02:06:00 — Rust baseline verification
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube
- Command: cargo check --workspace
- Status: PASS
- Relevant output: finished dev profile
- Known environmental limitations: None

## 2026-10-18 03:05:00 — Rust workspace regression
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube
- Command: cargo fmt --all -- --check
- Status: PASS
- Relevant output: no formatting differences
- Known environmental limitations: None

## 2026-10-18 03:06:00 — Rust workspace check
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube
- Command: cargo check --workspace
- Status: PASS
- Relevant output: finished dev profile
- Known environmental limitations: None

## 2026-10-18 03:07:00 — Rust workspace tests
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube
- Command: cargo test --workspace
- Status: PASS
- Relevant output: mutsukitube-anime 10 unit + 15 integration passed; storage 2; youtube 4; desktop 0
- Known environmental limitations: native_pagination test ignored (requires live YouTube network)

## 2026-10-18 03:08:00 — Go regression
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins
- Command: go test ./...
- Status: PASS
- Relevant output: all packages ok
- Known environmental limitations: None

## 2026-10-18 03:08:30 — Go vet
- Working directory: D:\Projects\mutsukitube-folder\mutsukitube-anime-plugins
- Command: go vet ./...
- Status: PASS
- Relevant output: no findings
- Known environmental limitations: None

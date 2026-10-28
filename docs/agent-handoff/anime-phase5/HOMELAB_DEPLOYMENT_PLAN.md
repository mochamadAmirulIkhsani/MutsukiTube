# MutsukiTube Phase 5 — ASUS X555BP Static Registry Deployment Plan

Status: PREPARATION ONLY — no server-side change has been performed.
Approval boundary: CP-00 authorized read-only SSH inspection only. Every
mutation below requires explicit operator approval first.

## 1. Verified environment (read-only inspection)

- Host: `X555BP` (Debian GNU/Linux 13 trixie, kernel 6.12.111+deb13-amd64, x86_64)
- SSH: `ikhsan@192.168.18.11:22` with existing Ed25519 key (`publickey` works)
- Memory: 10 GiB total, ~9.4 GiB available
- Disk: `/` 103 GiB (90 GiB free), `/srv/storage` 916 GiB (865 GiB free)
- Docker: `docker` binary present, `docker compose` v5.6.0 present
- Docker socket: **not accessible** to `ikhsan` (user not in `docker` group)
- `sudo -n` fails: **no passwordless sudo** (ikhsan is in `sudo` group)
- No system nginx, caddy, or apache installed
- Installed helpers: `python3`, `busybox` present in `/usr/bin`

### Existing Docker workloads (compose dirs under `/srv/docker`, preserved)

- `homeassistant` (port 8123)
- `jellyfin` (port 8096)
- `immich`
- `pihole` (DNS on 53)

### Occupied TCP ports

`22` (ssh), `53` (pihole DNS), `139`/`445` (samba), `8080`, `8096`
(jellyfin), `8123` (homeassistant), `9090`, `18554` (loopback), `18555`.

### Candidate registry directories (both `ikhsan`-owned, no root needed)

- Preferred: `/srv/storage/nas/mutsukitube-plugins` (865 GiB free, group writable)
- Alternative: `~/.local/share/mutsukitube-plugins`

## 2. Design decision

There is no existing Caddy/Nginx to reuse, the Docker socket is unavailable to
the operator account, and there is no passwordless sudo. The lowest-risk path
that respects "static artifacts only" and the constrained-hardware rule is a
**user-scoped `busybox httpd` static file server on port 8088**. It:

- serves files only (no scraping, no proxy, no database, no compilation),
- needs no root and no new packages,
- has minimal CPU/RAM footprint,
- keeps all anime work on the desktop.

Port `8088` is currently unused. No port forwarding or router change is planned;
initial scope is LAN only. Public HTTPS is a separate, later milestone.

## 3. Exact server changes (all require approval)

### 3.1 Registry directory

```bash
mkdir -p /srv/storage/nas/mutsukitube-plugins/releases
chmod 755 /srv/storage/nas/mutsukitube-plugins
```

### 3.2 Layout (immutable version directories)

```
/srv/storage/nas/mutsukitube-plugins/
├── manifest.json
├── manifest.sig
└── releases/
    └── v0.1.0/
        ├── otakudesu-windows-amd64.exe
        ├── samehadaku-windows-amd64.exe
        ├── nimegami-windows-amd64.exe
        ├── ylnime-windows-amd64.exe
        ├── moenime-windows-amd64.exe
        ├── astronime-windows-amd64.exe
        └── source-gpl.tar.gz
```

### 3.3 Static server (recommended: user systemd unit)

`~/.config/systemd/user/mutsukitube-registry.service`:

```ini
[Unit]
Description=MutsukiTube static plugin registry (read-only)
After=network-online.target

[Service]
Type=simple
WorkingDirectory=/srv/storage/nas/mutsukitube-plugins
ExecStart=/usr/bin/busybox httpd -f -p 0.0.0.0:8088 -h /srv/storage/nas/mutsukitube-plugins
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now mutsukitube-registry.service
```

Boot persistence across reboots without an interactive login additionally needs
`loginctl enable-linger ikhsan` (requires sudo, operator approval).

### 3.4 Docker alternative (only if approved and socket access granted)

```bash
docker run -d --name mutsukitube-registry --restart unless-stopped \
  -p 8088:80 \
  -v /srv/storage/nas/mutsukitube-plugins:/usr/share/nginx/html:ro \
  nginx:alpine
```

This requires adding `ikhsan` to the `docker` group (sudo) and reloading login.
Not recommended given no passwordless sudo.

## 4. Publishing a release (after approval)

1. Build and sign locally in the private Go repo (never on ASUS).
2. Upload artifacts first, then the manifest last (atomic-ish publication).
3. Verify uploaded hashes before activating the manifest.

```bash
# upload (from Windows or a build host)
scp -r dist/v0.1.0 ikhsan@192.168.18.11:/srv/storage/nas/mutsukitube-plugins/releases/
scp dist/manifest.json dist/manifest.sig ikhsan@192.168.18.11:/srv/storage/nas/mutsukitube-plugins/

# verify on the server (read-only)
sha256sum /srv/storage/nas/mutsukitube-plugins/releases/v0.1.0/*.exe
```

## 5. Verification commands (read-only)

```bash
# from Windows PC on the LAN
curl -sS http://192.168.18.11:8088/manifest.json -o manifest.json
curl -sS http://192.168.18.11:8088/manifest.sig -o manifest.sig
# verify with the pinned public key using the Go tooling or the Rust updater

# confirm no scraper/Go runtime is running on ASUS
ssh ikhsan@192.168.18.11 "pgrep -a -f 'otakudesu|samehadaku|ylnime|moenime|nimegami|astronime' || echo none"
```

## 6. Rollback steps

```bash
systemctl --user disable --now mutsukitube-registry.service
# repository metadata is unchanged; remove files only if explicitly requested
```

For a bad release: restore the previous `manifest.json`/`manifest.sig` and keep
old `releases/vX.Y.Z/` directories (immutable, never overwritten). Existing
desktop clients keep using their installed plugins offline.

## 7. Impact on existing services

- New listening port `8088` on the LAN interface only.
- No changes to Docker containers, firewall, router, DNS, or Samba.
- No anime scraper, media proxy, transcoder, database, or Go runtime added.
- Disk usage: ~7 MB per provider binary (~45 MB per release) plus source archive.

## 8. Blocked items (require operator action)

- Approval to create the registry directory and run a static server.
- Providing a production Ed25519 public key (pinned into the app build).
- Public HTTPS endpoint: not designed here, separate approval needed.
- Docker access (only if the Docker alternative is chosen).

Do not treat any of the above as deployed until verified over the LAN.

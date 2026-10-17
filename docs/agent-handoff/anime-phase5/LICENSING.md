# MutsukiTube Phase 5 — Licensing and Corresponding-Source Plan

## Repository licenses (verified in-tree)

- Go plugin repository `mutsukitube-anime-plugins`: **GPL-3.0** (`LICENSE`,
  GNU GENERAL PUBLIC LICENSE Version 3).
- Rust `mutsukitube`: see repository `LICENSE`; desktop code is distributed
  separately from the Go plugins.

## Third-party dependencies of the Go plugins

From `go.mod`:

| Module | License |
| --- | --- |
| `github.com/PuerkitoBio/goquery` | BSD-3-Clause |
| `github.com/andybalholm/cascadia` | BSD-3-Clause |
| `golang.org/x/net` | BSD-3-Clause |

The provider scrapers were developed with Kyoko/Aikatsu as behavioural
references. If any GPL-covered code was adapted from those projects, the
distributed plugin binaries are GPL-covered derivatives and recipients must be
able to obtain the corresponding source.

## Obligations for distributing plugin binaries

Because the plugin executables are offered for download from the registry:

1. Provide the **complete corresponding source** for the exact release build,
   including build scripts needed to reproduce the binaries.
2. Provide the **GPL-3.0 license text** and any required attribution/notices.
3. Do not add restrictions beyond GPL-3.0 on the covered code.

The private development repository does **not** have to be made public. A
source-release archive is sufficient, published alongside the artifacts.

## Planned release artifacts per version

```
releases/vX.Y.Z/
  <provider>-windows-amd64.exe     (and linux where built)
  source-gpl.tar.gz                (corresponding source + build info)
  NOTICE                           (third-party licenses and attribution)
  LICENSE                          (GPL-3.0 text)
```

## Source archive contents (proposed)

- Go source for the released tag (all providers, `go.mod`/`go.sum`),
- `scripts/build.ps1` and the manifest/signing tooling,
- third-party license texts (`NOTICE`),
- a short `BUILDING.md` note describing the build command and toolchain
  (`go 1.24.0`).

## Exclusions (must never be packaged)

- SSH private keys, Ed25519 signing private keys, GitHub tokens,
  deployment credentials, unrelated private files.

## Status

- Source archive generation is **prepared/documented but not published**.
- No source release has been uploaded to the registry (deployment not approved).
- No private key material has been committed or logged.
- The private GitHub repository remains private; no visibility change was made.

## Approval required

Publishing `source-gpl.tar.gz`, `NOTICE`, and `LICENSE` into a release requires
the same operator approval as any other registry upload (CP-03 / CP-08).

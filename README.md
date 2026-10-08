<div align="center">
  <img src="./Mutsuki.png" alt="MutsukiTube" width="220">

  # MutsukiTube

  **A cute and funny lightweight YouTube app for desktop, built with Rust.**
</div>

MutsukiTube is a lightweight desktop application for searching and watching
YouTube videos with a cute and fun interface. It is built with Rust and Slint,
using `yt-dlp` as the video metadata provider and `mpv` as the external media
player.

## Features

- Search for YouTube videos from the desktop application.
- Display video titles, channels, durations, and thumbnails.
- Open and play videos using mpv.
- Modular Rust workspace with separate crates for the core, YouTube provider,
  player, and desktop application.

## Prerequisites

Make sure the following tools are installed:

- [Rust](https://www.rust-lang.org/tools/install) and Cargo.
- [`yt-dlp`](https://github.com/yt-dlp/yt-dlp), available in your `PATH`.
- [`mpv`](https://mpv.io/installation/), available in your `PATH`.

## Running the application

Clone the repository, then run the desktop package:

```bash
cargo run -p mutsukitube-desktop
```

## Project structure

```text
.
├── apps/
│   └── desktop/                 # Desktop application and Slint UI
├── crates/
│   ├── mutsukitube-core/        # Video model and provider trait
│   ├── mutsukitube-player/      # mpv integration
│   └── mutsukitube-youtube/     # YouTube metadata provider via yt-dlp
├── Cargo.toml                   # Rust workspace configuration
└── Mutsuki.png                  # Project icon/thumbnail
```

## Status

MutsukiTube is currently in early development. Its API and interface may change
as new features are added.

## License

The project license has not been decided yet.

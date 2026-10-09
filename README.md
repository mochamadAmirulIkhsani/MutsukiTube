<div align="center">
  <img src="./Mutsuki.png" alt="MutsukiTube" width="220">

# MutsukiTube

  **A cute and funny lightweight YouTube app for desktop, built with Rust.**

Watch YouTube videos without running an entire browser. MutsukiTube focuses on native rendering, lightweight architecture, and a straightforward desktop experience.

![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange?logo=rust)
![Slint](https://img.shields.io/badge/UI-Slint-blue)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D4?logo=windows)
![Status](https://img.shields.io/badge/Status-Active%20Development-yellow)
![Version](https://img.shields.io/badge/Version-0.1.0-blue)

> [!IMPORTANT]
> **MutsukiTube is under active development.** Features, architecture, and APIs may change. The application is not yet considered production-ready.
</div>

## 🌟 Highlights

- **Native desktop UI** — Built with [Slint](https://slint.dev/), without Electron or an embedded browser interface.
- **Embedded video playback** — Uses libmpv inside a native Windows video surface.
- **YouTube search** — Native YouTube search with optional yt-dlp fallback.
- **Local library** — Manage favorites, playlists, and watch history.
- **Responsive video rendering** — Native video surface that adjusts to window resizing.
- **Fullscreen playback** — Native fullscreen mode without recreating the player.
- **Playback controls** — Play, pause, stop, volume, seek, and replay.
- **Local persistence** — SQLite-backed application data and preferences.
- **Rust-based architecture** — Modular workspace with separate crates for application features.

## 📖 Overview

MutsukiTube is an experimental desktop YouTube client written primarily in Rust.

The project explores an alternative to browser-based video applications by combining a native user interface with embedded video playback.

Instead of rendering YouTube through a WebView, MutsukiTube uses a native Slint interface and libmpv for playback. This separation is intended to provide a more focused desktop experience with less browser-related overhead.

The project is currently developed and tested primarily on **Windows**.

### Why MutsukiTube?

Many desktop video applications use browser technologies for their entire interface.

MutsukiTube takes a different approach:

- Keep the interface native.
- Use Rust for application logic.
- Use libmpv for media playback.
- Store personal application data locally.
- Keep components modular and maintainable.

The goal is to build a capable YouTube desktop application without depending on a full browser rendering engine.

## ✨ Features

| Feature | Status |
| --- | --- |
| YouTube video search | ✅ Implemented |
| Native search provider | ✅ Implemented |
| yt-dlp search fallback | ✅ Implemented |
| Video thumbnails | ✅ Implemented |
| Embedded libmpv playback | ✅ Implemented |
| Play / Pause / Stop | ✅ Implemented |
| Volume control | ✅ Implemented |
| Watch history | ✅ Implemented |
| Favorites | ✅ Implemented |
| Local playlists | ✅ Implemented |
| Provider settings | ✅ Implemented |
| Native fullscreen | ✅ Implemented |
| Responsive video surface | ✅ Implemented |
| End-of-video detection | ✅ Implemented |
| Video replay | ✅ Implemented |
| Seek slider | ⚠️ Known issue |
| Minimize / restore optimization | 🚧 In progress |
| Keyboard shortcuts | 📋 Planned |
| Modern UI redesign | 📋 Planned |
| Cross-platform embedded playback | 📋 Planned |

## 🖥️ Preview

Application screenshots and demonstrations will be added as development progresses.

Current interface pages include:

- **Search** — Find YouTube videos.
- **Watch** — Play videos using embedded libmpv.
- **History** — Revisit previously watched videos.
- **Library** — Manage favorites and playlists.
- **Settings** — Configure the video search provider.

## 🏗️ Architecture

MutsukiTube is organized as a Rust Cargo workspace with multiple application crates.

```text
mutsukitube/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── handlers/
│       │   ├── native_window/
│       │   ├── player_controller/
│       │   └── main.rs
│       └── ui/
│           ├── components/
│           ├── pages/
│           └── app.slint
├── crates/
│   ├── mutsukitube-core/
│   ├── mutsukitube-youtube/
│   ├── mutsukitube-player/
│   ├── mutsukitube-embedded-player/
│   └── mutsukitube-storage/
├── vendor/
├── Cargo.toml
└── README.md
```

### Technology Stack

| Component | Technology |
| --- | --- |
| Language | Rust |
| UI framework | Slint |
| Embedded playback | libmpv / rsmpv |
| Video search | YouTube native provider / yt-dlp |
| Database | SQLite / rusqlite |
| Async runtime | Tokio |
| HTTP client | Reqwest |
| Native video surface | Win32 HWND |
| Build system | Cargo |

### Playback Architecture

```text
Slint Desktop UI
       |
       v
Player Controller
       |
       v
Embedded libmpv Wrapper
       |
       v
      libmpv
       |
       v
Native Win32 Video Surface
```

The desktop application manages user interactions and playback state, while libmpv handles media decoding and playback.

## 🚀 Getting Started

### Requirements

For the current Windows development build, you will need:

- Windows 10 or Windows 11 (64-bit)
- Rust toolchain with the MSVC target
- Visual Studio C++ Build Tools
- libmpv runtime DLL and MSVC import library
- Internet connection

Additional requirements may apply depending on the selected YouTube provider.

### Build from Source

Clone the MutsukiTube repository and navigate to its root directory.

Install the Rust toolchain through [rustup](https://rustup.rs/) if necessary.

Check that Cargo is available:

```powershell
cargo --version
rustc --version
```

Verify the desktop application:

```powershell
cargo check -p mutsukitube-desktop
```

Run the application:

```powershell
cargo run -p mutsukitube-desktop
```

### libmpv Configuration

MutsukiTube currently uses libmpv for native Windows playback.

The development environment must provide the appropriate 64-bit libmpv runtime DLL and an MSVC-compatible `mpv.lib` import library.

Depending on the local build configuration, the libmpv library directory may need to be provided to Cargo, and the runtime DLL must be discoverable when the application starts.

**Note:** Large native binaries are not necessarily distributed through this repository. Prebuilt installation packages are not yet available.

## 🎮 Usage

1. Launch MutsukiTube.
2. Search for a video using the search page.
3. Select a video to open the Watch page.
4. Click **Play Video** to start playback.
5. Use the playback controls to pause, resume, stop, or adjust volume.
6. Enter fullscreen mode using the **Fullscreen** button.
7. Save videos to Favorites or a playlist.
8. Access previously watched videos through History.

When a video finishes naturally, MutsukiTube detects the end-of-file event and enables the **Replay** action.

## 🐛 Known Issues

MutsukiTube is still experimental, and several areas require further testing.

### Platform Support

The embedded player implementation currently targets Windows through Win32 HWND integration.

Other platforms are not yet supported by the same embedded playback implementation.

## 🗺️ Roadmap

### Native Player Improvements

- [x] Embedded libmpv integration
- [x] Native Win32 video surface
- [x] Playback controls
- [x] End-of-file detection
- [x] Replay support
- [x] Fullscreen playback
- [x] Responsive video resizing
- [ ] Complete minimize and restore optimization
- [ ] Resolve playback progress slider issues
- [ ] Implement keyboard shortcuts

### Interface Improvements

- [ ] Modernize the Slint interface
- [ ] Improve playback control layout
- [ ] Add video loading indicators
- [ ] Improve fullscreen interaction
- [ ] Introduce reusable UI components

### Future Development

- [ ] Improve error handling and recovery
- [ ] Optimize thumbnail loading and caching
- [ ] Improve search and library experience
- [ ] Explore cross-platform playback support
- [ ] Provide distributable desktop builds

This roadmap is subject to change as development continues.

## 🛠️ Development

To check all workspace packages:

```powershell
cargo check --workspace
```

Format the Rust source code:

```powershell
cargo fmt --all
```

Run workspace tests:

```powershell
cargo test --workspace
```

Run the desktop application:

```powershell
cargo run -p mutsukitube-desktop
```

**Note:** Tests that link against libmpv require the native development libraries to be configured correctly.

## 🤝 Contributing

MutsukiTube is an ongoing personal open-source development project.

Bug reports, suggestions, and contributions are welcome.

If you discover an issue or have an improvement in mind, feel free to open a GitHub Issue.

For code contributions, please describe the change and its purpose clearly in your pull request.

## ⚠️ Disclaimer

MutsukiTube is an independent, unofficial project and is not affiliated with, endorsed by, or sponsored by YouTube or Google.

YouTube is a trademark of Google LLC.

Users are responsible for complying with applicable laws, content licenses, and the terms of services they access.

## 📄 License

License information will be provided when the project license has been finalized.

---

<div align="center">

**Built with Rust, Slint, and libmpv.**

*Lightweight by design. Native by nature.*

</div>

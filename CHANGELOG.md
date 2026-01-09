# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Demo TUI Mode**: Conference-ready visual demo with ratatui
  - Live race visualization showing gity vs vanilla git side-by-side
  - Progress bars with real-time timing updates
  - 5-act narrative workflow (Morning Check-in, Coding Session, Commit Flow, Branch Switching, Background Magic)
  - Summary table with speedup metrics and time saved estimates
  - Color-coded results (green for gity, red for baseline)
  - Automatic fallback to classic text mode when terminal unavailable

- **Performance Improvements**:
  - Added mimalloc global allocator for faster memory allocation
  - Adaptive poll interval (50ms active, 250ms idle, 500ms very idle)
  - Memory-mapped fsmonitor cache for zero-copy responses

### Changed

- Demo now defaults to TUI mode; use `--classic` flag for text-based output

## [0.1.0] - 2024-12-09

### Added

- Initial release of Gity
- Cross-platform support (Linux, macOS, Windows)
- Background daemon with file watching via OS-native watchers (inotify, FSEvents, ReadDirectoryChangesW)
- Git fsmonitor protocol integration for accelerated `git status`
- Repository registration and management (`gity register`, `gity unregister`, `gity list`)
- Health checking and diagnostics (`gity health`)
- Background maintenance scheduling
- Status caching for instant responses
- System tray UI (optional)
- IPC communication between CLI and daemon
- Persistent storage with sled database

[Unreleased]: https://github.com/neul-labs/gity/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/neul-labs/gity/releases/tag/v0.1.0

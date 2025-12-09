# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

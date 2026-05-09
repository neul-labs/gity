# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-05-10

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

- **CI/CD & Supply Chain Security**:
  - OIDC-based Trusted Publishing for crates.io via `rust-lang/crates-io-auth-action`
  - OIDC-based Trusted Publishing for PyPI via `pypa/gh-action-pypi-publish`
  - OIDC-based Trusted Publishing for npm with automatic provenance attestations
  - GitHub artifact attestations for all release binaries via `actions/attest-build-provenance`
  - Consolidated CI workflow with lint, test, audit, and auto-tagging

- **Documentation & SEO**:
  - SEO-optimized READMEs for all workspace crates
  - SEO-optimized PyPI and npm package READMEs with install instructions
  - Updated package metadata (keywords, classifiers, descriptions)
  - Supply chain security section in README and docs

### Changed

- Demo now defaults to TUI mode; use `--classic` flag for text-based output
- Release workflow now publishes workspace crates in strict dependency order
- npm publishing now uses Node.js 24+ with native OIDC support

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

[Unreleased]: https://github.com/neul-labs/gity/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/neul-labs/gity/releases/tag/v0.1.2
[0.1.0]: https://github.com/neul-labs/gity/releases/tag/v0.1.0

# gity

**Make large Git repositories feel instant.**

[![Crates.io](https://img.shields.io/crates/v/gity)](https://crates.io/crates/gity)
[![Documentation](https://img.shields.io/badge/docs-neullabs.com-blue)](http://docs.neullabs.com/gity)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Cross-platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue)](http://docs.neullabs.com/gity)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity?

Gity is a lightweight, cross-platform daemon that accelerates everyday Git operations on large repositories. It watches your working tree with OS-native file system monitors, integrates with Git's `fsmonitor` protocol, and caches status results so that `git status` returns in milliseconds instead of seconds — even in monorepos with millions of files.

## Quick Start

```bash
# Install
cargo install gity

# Register your large repo (one-time setup)
gity register /path/to/large-repo

# That's it! Git commands are now accelerated
cd /path/to/large-repo
git status  # Fast!
```

The daemon starts automatically when needed. For manual control:

```bash
gity daemon start   # Start in background
gity daemon stop    # Stop gracefully
gity list           # See registered repos
gity health <repo>  # Check repo health
```

## Features

- **File watching** — Detects changes instantly via `inotify`, `FSEvents`, or `ReadDirectoryChangesW`
- **fsmonitor integration** — Tells Git exactly what changed so it skips full tree scans
- **Background maintenance** — Runs `git maintenance` during idle periods to keep objects fresh
- **Status caching** — Serves results instantly when nothing changed
- **System tray** — Optional menu-bar UI (`cargo install gity --features tray`)
- **Cross-platform** — Single binary for Linux, macOS, and Windows

## Platform Support

| Platform | File Watcher | Status |
|----------|--------------|--------|
| Linux    | inotify      | Full support |
| macOS    | FSEvents     | Full support |
| Windows  | ReadDirectoryChangesW | Full support |

## Requirements

- Git 2.37+ (for fsmonitor-watchman protocol v2)
- Rust 1.75+ (to build from source)
- Linux, macOS, or Windows

## Installation

### From crates.io

```bash
cargo install gity
```

### With system tray

```bash
cargo install gity --features tray
```

### Pre-built binaries

See [GitHub Releases](https://github.com/neul-labs/gity/releases) for `.tar.gz`, `.zip`, `.deb`, `.pkg`, and `.msi` installers.

## Configuration

Gity stores data in `$GITY_HOME` (defaults to `~/.gity` on Unix, `%APPDATA%\Gity` on Windows):

```
~/.gity/
├── data/
│   ├── sled/           # Metadata database
│   └── status_cache/   # Cached status results
└── logs/
    └── daemon.log      # Daemon logs
```

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `GITY_HOME` | Data directory | `~/.gity` |
| `GITY_DAEMON_ADDR` | IPC address | `tcp://127.0.0.1:7557` |

## Documentation

- [Architecture](https://github.com/neul-labs/gity/blob/main/docs/architecture.md) — System design and data flow
- [fsmonitor integration](https://github.com/neul-labs/gity/blob/main/docs/fsmonitor.md) — Protocol details and edge cases
- [Commands](https://github.com/neul-labs/gity/blob/main/docs/commands.md) — Complete CLI reference
- [Alternatives](https://github.com/neul-labs/gity/blob/main/docs/alternatives.md) — Comparison with other approaches

## License

MIT

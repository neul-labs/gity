# gity-daemon

**Background daemon that makes large Git repositories feel instant.**

[![Crates.io](https://img.shields.io/crates/v/gity-daemon)](https://crates.io/crates/gity-daemon)
[![Documentation](https://docs.rs/gity-daemon/badge.svg)](https://docs.rs/gity-daemon)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-daemon?

`gity-daemon` is the core engine behind [gity](https://crates.io/crates/gity). It runs in the background, watches your repositories for file changes, caches `git status` results, and schedules `git maintenance` during idle periods. By integrating with Git's native `fsmonitor` protocol, it tells Git exactly which files changed — so `git status` skips the full tree scan and returns in milliseconds, even in monorepos with millions of files.

## Features

- **File system watching** — OS-native watchers (`inotify`, `FSEvents`, `ReadDirectoryChangesW`) per repository
- **fsmonitor protocol v2** — Answers Git's "what changed?" queries in <1 ms
- **Status caching** — Remembers `git status` output and serves it instantly when nothing changed
- **Background maintenance** — Runs `git maintenance` prefetch, loose-objects, and incremental-repack automatically
- **IPC server** — Async NNG-based server for CLI and tray communication
- **Persistent state** — sled-backed storage for registrations and cache across restarts
- **Cross-platform** — Linux, macOS, Windows

## Usage

This crate is primarily used internally by the `gity` binary. Most users should install `gity` directly:

```bash
cargo install gity
gity register /path/to/large-repo
```

If you are building a tool that embeds the daemon:

```rust
use gity_daemon::Daemon;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let daemon = Daemon::new().await?;
    daemon.run().await?;
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-daemon = "0.1.2"
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Full support |
| macOS    | Full support |
| Windows  | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — User-facing CLI and binary
- [gity-watch](https://crates.io/crates/gity-watch) — File system watcher used by this daemon
- [gity-git](https://crates.io/crates/gity-git) — Git operations library used for maintenance
- [gity-storage](https://crates.io/crates/gity-storage) — Persistent storage for daemon state
- [gity-ipc](https://crates.io/crates/gity-ipc) — IPC protocol for CLI/daemon communication

## License

MIT

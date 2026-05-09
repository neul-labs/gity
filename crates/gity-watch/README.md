# gity-watch

**Cross-platform file system watcher optimized for large Git repositories.**

[![Crates.io](https://img.shields.io/crates/v/gity-watch)](https://crates.io/crates/gity-watch)
[![Documentation](https://docs.rs/gity-watch/badge.svg)](https://docs.rs/gity-watch)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-watch?

`gity-watch` provides efficient, recursive directory monitoring for the [gity](https://crates.io/crates/gity) daemon. It abstracts OS-native watchers — `inotify` on Linux, `FSEvents` on macOS, `ReadDirectoryChangesW` on Windows — into a single async stream of debounced change events. Designed to stay lightweight even when watching trees with millions of files.

## Features

- **OS-native backends** — `inotify`, `FSEvents`, `ReadDirectoryChangesW` (no polling)
- **Recursive watching** — Auto-follows nested directories
- **Debounced event stream** — Batches rapid changes to reduce noise
- **Async delivery** — Tokio-backed channel interface
- **Respects `.gitignore`** — Skips ignored paths automatically
- **Cross-platform** — Identical API across Linux, macOS, and Windows

## Usage

```rust
use gity_watch::Watcher;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(128);
    let _watcher = Watcher::new("/path/to/repo", tx).await?;

    while let Some(event) = rx.recv().await {
        println!("Changed: {:?}", event.paths);
    }
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-watch = "0.1.2"
```

## Platform Support

| Platform | Backend               | Status       |
|----------|-----------------------|--------------|
| Linux    | inotify               | Full support |
| macOS    | FSEvents              | Full support |
| Windows  | ReadDirectoryChangesW | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — Git acceleration daemon that uses this watcher
- [gity-daemon](https://crates.io/crates/gity-daemon) — Daemon that coordinates watchers per repo
- [notify](https://crates.io/crates/notify) — The underlying cross-platform notify crate

## License

MIT

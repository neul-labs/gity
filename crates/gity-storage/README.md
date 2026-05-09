# gity-storage

**Embedded key-value storage for Git developer tools, powered by sled.**

[![Crates.io](https://img.shields.io/crates/v/gity-storage)](https://crates.io/crates/gity-storage)
[![Documentation](https://docs.rs/gity-storage/badge.svg)](https://docs.rs/gity-storage)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-storage?

`gity-storage` is the persistent metadata layer for the [gity](https://crates.io/crates/gity) daemon. It uses the [sled](https://crates.io/crates/sled) embedded database to store repository registrations, status cache entries, and maintenance history — all without requiring an external database server. Ideal for CLI tools and background daemons that need durable, structured local storage.

## Features

- **Zero-config embedded DB** — sled handles persistence, crash recovery, and concurrency
- **Structured records** — Repository metadata, cache entries, and maintenance logs as typed values
- **Atomic batches** — Multi-key transactions for consistent updates
- **Small footprint** — No background processes, no ports, no admin
- **Cross-platform** — Linux, macOS, Windows

## Usage

```rust
use gity_storage::Storage;

fn main() -> anyhow::Result<()> {
    let db = Storage::open("~/.gity/data")?;
    db.register_repo("/my/repo")?;
    println!("Registered repos: {:?}", db.list_repos()?);
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-storage = "0.1.2"
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Full support |
| macOS    | Full support |
| Windows  | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — Main binary that uses this storage layer
- [gity-daemon](https://crates.io/crates/gity-daemon) — Daemon that reads/writes storage on every operation
- [sled](https://crates.io/crates/sled) — The underlying embedded database engine

## License

MIT

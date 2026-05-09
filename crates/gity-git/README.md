# gity-git

**Safe, high-level Git operations library for Rust developer tools.**

[![Crates.io](https://img.shields.io/crates/v/gity-git)](https://crates.io/crates/gity-git)
[![Documentation](https://docs.rs/gity-git/badge.svg)](https://docs.rs/gity-git)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/neul-labs/gity/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

## What is gity-git?

`gity-git` wraps [libgit2](https://libgit2.org) (via the [`git2`](https://crates.io/crates/git2) crate) into an ergonomic, error-typed API for tools that need to inspect, maintain, or accelerate Git repositories. It powers the [gity](https://crates.io/crates/gity) daemon's repository introspection, status queries, and background maintenance tasks.

## Features

- **Repository discovery** — Find `.git` roots from any path, validate repo health
- **Fast status queries** — Enumerate changed/staged/untracked files without spawning `git` subprocesses
- **Maintenance tasks** — Run `git maintenance` sub-operations programmatically
- **Branch & remote introspection** — List branches, track upstreams, read remote configs
- **Vendored libgit2** — No system dependency on `libgit2`; builds out of the box
- **Cross-platform** — Linux, macOS, Windows

## Usage

```rust
use gity_git::{Repository, StatusOptions};

fn main() -> anyhow::Result<()> {
    let repo = Repository::open("/path/to/repo")?;
    let status = repo.status(StatusOptions::default())?;
    println!("Changed files: {}", status.changed.len());
    Ok(())
}
```

## Installation

```toml
[dependencies]
gity-git = "0.1.2"
```

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Full support |
| macOS    | Full support |
| Windows  | Full support |

## See Also

- [gity](https://crates.io/crates/gity) — Main binary that accelerates Git operations
- [gity-daemon](https://crates.io/crates/gity-daemon) — Daemon that schedules maintenance via this library
- [git2](https://crates.io/crates/git2) — The underlying libgit2 bindings

## License

MIT
